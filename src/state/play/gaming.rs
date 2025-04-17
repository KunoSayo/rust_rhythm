use crate::engine::global::STATIC_DATA;
use crate::engine::renderer::texture_renderer::TextureRenderer;
use crate::engine::{EasyGuiExt, GameState, LoopState, StateData, StateEvent, Trans};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::play::{Gaming, NoteResult, PlayingNoteType};
use crate::game::beatmap::{GamePos, FOUR_KEY_X};
use crate::game::render::NoteRenderer;
use crate::game::song::SongInfo;
use crate::game::{get_play_rect, secs_to_offset_type};
use anyhow::anyhow;
use egui::{Align, Color32, Context, Frame, Layout, Pos2, Rect, RichText, Stroke, Vec2};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStreamHandle, Sink, Source};
use std::io::{Cursor, Read};
use std::ops::{Add, Deref};
use std::time::Duration;
use egui::ahash::HashMap;
use tokio::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::keyboard::{Key, KeyCode, PhysicalKey};

#[derive(Default)]
pub struct HitFeedback {
    last_result: Option<(NoteResult, Instant)>,
}

pub struct GamingState {
    pub total_duration: Duration,
    pub start_time: Instant,
    hit_feedback: HitFeedback,
    /// the pointer pos when last update.
    gaming: Gaming,
    game_rect: Rect,
    sink: Sink,
}

impl GamingState {
    pub(crate) fn get_game_time(&self) -> f32 {
        if self.sink.len() == 0 {
            return self.total_duration.as_secs_f32();
        }
        self.sink.get_pos().as_secs_f32() - 3.0 * (self.sink.len().max(1) - 1) as f32
    }

    pub fn new(
        handle: OutputStreamHandle,
        song_info: &SongInfo,
        beatmap_file: SongBeatmapFile,
    ) -> anyhow::Result<Self> {
        let sink = Sink::try_new(&handle)?;

        let mut buf = vec![];
        let mut file = std::fs::File::open(&song_info.bgm_file)?;
        file.read_to_end(&mut buf)?;

        let buf = Cursor::new(buf);
        let decoder = Decoder::new(buf.clone())?;

        let samples = decoder.convert_samples::<f32>();

        let total_duration = samples
            .total_duration()
            .ok_or(anyhow!("No audio duration"))?
            .add(Duration::from_secs_f32(3.0));

        // append blank
        {
            sink.append(SamplesBuffer::new(
                samples.channels(),
                samples.sample_rate(),
                &vec![0i16; (samples.channels() as u32 * samples.sample_rate()) as usize * 3][..],
            ));
            sink.pause();
            sink.try_seek(Duration::ZERO).expect("?");
        }
        sink.append(samples);
        sink.pause();
        sink.try_seek(Duration::ZERO).expect("?");
        let vol = STATIC_DATA
            .cfg_data
            .write()
            .map_err(|e| anyhow!("Cannot read lock for {:?}", e))?
            .get_f32_def("bgm_vol", 1.0);
        sink.set_volume(vol);

        let this = Self {
            total_duration,
            start_time: Instant::now(),
            hit_feedback: Default::default(),
            gaming: Gaming::load_game(beatmap_file),
            game_rect: Rect::ZERO,
            sink,
        };
        Ok(this)
    }

    fn update_game_region(&mut self, size: PhysicalSize<u32>) {
        // we are 4:3 game
        self.game_rect = get_play_rect(Rect::from_min_max(
            Pos2::ZERO,
            Pos2::new(size.width as f32, size.height as f32),
        ))
    }
}

impl GameState for GamingState {
    fn start(&mut self, s: &mut StateData) -> LoopState {
        log::info!("Gaming state start!");
        self.sink.play();
        self.start_time = Instant::now();
        if let Some(gpu) = s.app.gpu.as_ref() {
            self.update_game_region(gpu.get_screen_size().into());
        }
        LoopState::POLL
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        let mut trans = Trans::None;

        let cur_input = &s.app.inputs.cur_frame_input;
        if cur_input
            .pressing
            .contains(&PhysicalKey::Code(KeyCode::Escape))
        {
            trans = Trans::Pop;
        }
        // in fact, we do update in render, for we are polling.
        (trans, LoopState::POLL)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut trans = Trans::None;
        let game_time = self.get_game_time();
        self.gaming.tick(
            game_time,
            Some(|note: PlayingNoteType<'_>, result| {
                self.hit_feedback.last_result = Some((result, Instant::now()))
            }),
        );
        let gpu = s.app.gpu.as_mut().unwrap();
        let mut nr = s.app.world.fetch_mut::<NoteRenderer>();
        for (timing_group, x) in self.gaming.normal_notes.iter().enumerate() {
            let current_y = self.gaming.raw_file.timing_group.get_gameplay_y_f32(
                game_time,
                timing_group as u8,
                self.gaming.ops.default_view_time,
            );
            let (normal_a, normal_b) = x.get_play_notes().as_slices();
            nr.collect_playing_notes(normal_a, gpu.get_screen_size_f32(), current_y);
            nr.collect_playing_notes(normal_b, gpu.get_screen_size_f32(), current_y);
        }
        for (timing_group, x) in self.gaming.long_notes.iter().enumerate() {
            let current_y = self.gaming.raw_file.timing_group.get_gameplay_y_f32(
                game_time,
                timing_group as u8,
                self.gaming.ops.default_view_time,
            );
            let (normal_a, normal_b) = x.get_play_notes().as_slices();
            nr.collect_playing_notes(normal_a, gpu.get_screen_size_f32(), current_y);
            nr.collect_playing_notes(normal_b, gpu.get_screen_size_f32(), current_y);
        }

        let tr = s.app.world.fetch::<TextureRenderer>();
        nr.render(
            gpu,
            s.app.render.as_mut().unwrap(),
            tr.deref(),
            &self.game_rect,
        );

        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                ui.painter().hline(
                    ui.max_rect().x_range(),
                    self.game_rect.center().y,
                    Stroke::new(1.0, Color32::WHITE),
                );

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.no_select_text(
                        format!("{}", self.gaming.combo_counter.get_combo()),
                        [300.0, 100.0],
                    );
                    if let Some(last_result) = self.hit_feedback.last_result {
                        if last_result.1.elapsed().as_secs_f32() <= 3.0 {
                            let elap = last_result.1.elapsed().as_secs_f32().min(1.0);
                            ui.no_select_text(
                                RichText::new(format!("{:?}", last_result.0))
                                    .size(36.0 * (2.0 - elap)),
                                Vec2::new(300.0, 100.0),
                            );
                        }
                    }
                });
            });
        trans
    }

    fn on_event(&mut self, s: &mut StateData, event: StateEvent) {
        match event {
            StateEvent::Window(event, time) => match event {
                WindowEvent::KeyboardInput {
                    device_id,
                    event,
                    is_synthetic,
                } => match event.physical_key {
                    PhysicalKey::Code(code) => match code {
                        _ => {
                            let input_game_time =
                                self.get_game_time() - time.elapsed().as_secs_f32();

                            let input_x = match code {
                                KeyCode::KeyD => FOUR_KEY_X[0],
                                KeyCode::KeyF => FOUR_KEY_X[1],
                                KeyCode::KeyJ => FOUR_KEY_X[2],
                                KeyCode::KeyK => FOUR_KEY_X[3],
                                _ => return,
                            };
                            let game_input =
                                GamePos::new(input_x, secs_to_offset_type(input_game_time));
                            if event.state.is_pressed() {
                                if let Some(result) = self
                                    .gaming
                                    .process_input(game_input, ((input_x + 0.75) * 4.0) as _)
                                {
                                    self.hit_feedback.last_result = Some((result, Instant::now()));
                                }
                            } else {
                                self.gaming
                                    .process_input_leave(game_input, ((input_x + 0.75) * 4.0) as _);
                            }
                        }
                    },
                    PhysicalKey::Unidentified(_) => {}
                },
                WindowEvent::Resized(size) => self.update_game_region(*size),
                _ => {}
            },
            _ => {}
        }
    }
}
