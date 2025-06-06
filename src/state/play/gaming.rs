use crate::engine::global::STATIC_DATA;
use crate::engine::renderer::texture_renderer::TextureRenderer;
use crate::engine::sources::ControlledBufferHandle;
use crate::engine::{EasyGuiExt, GameState, LoopState, OutputStreamHandle, ResourceLocation, StateData, StateEvent, Trans};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::play::{Gaming, NoteHitResult, NoteResult, PlayingNoteType};
use crate::game::beatmap::summary::BeatmapPlayResult;
use crate::game::beatmap::{GamePos, FOUR_KEY_X};
use crate::game::render::NoteRenderer;
use crate::game::song::SongInfo;
use crate::game::{get_play_rect, secs_to_offset_type, GameTimeType, OffsetType};
use crate::state::play::end::EndResultState;
use anyhow::anyhow;
use egui::{
    Align, Color32, Context, Frame, Layout, Pos2, Rect, RichText, Stroke, TextStyle, Vec2, Widget,
};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, Sink, Source};
use std::io::{Cursor, Read};
use std::ops::{Add, Deref};
use std::time::Duration;
use tokio::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Default)]
pub struct HitFeedback {
    last_result: Option<(NoteHitResult, Instant)>,
}

/// We delay the score display in 3s
/// Every score will add to the current in 3s
#[derive(Default)]
struct ScoreDisplay {
    /// The last recorded score, indicate whether should record new delayed score.
    current: u32,
    /// The base score for display
    display_base: u32,
    /// The score delayed add to current, (origin score delta, add time)
    delayed: Vec<(u32, Instant)>,
}

impl ScoreDisplay {
    fn mark_score(&mut self, score: u32) -> u32 {
        if self.current != score {
            self.delayed.push((score - self.current, Instant::now()));
            self.current = score;
        }

        let mut display = self.display_base;
        self.delayed.retain(|(delta, added_time)| {
            let passed = added_time.elapsed().as_secs_f32();
            if passed >= 1.0 {
                display += *delta;
                self.display_base += *delta;
                false
            } else {
                display += (*delta as f32 * passed / 1.0).round() as u32;
                true
            }
        });

        display
    }
}

pub struct GamingState {
    pub total_duration: Duration,
    pub start_time: Instant,
    hit_feedback: HitFeedback,
    /// the pointer pos when last update.
    gaming: Box<Gaming>,
    game_rect: Rect,
    sink: ControlledBufferHandle,
    score_display: ScoreDisplay,
    end_remaining: Option<f32>,
}

impl GamingState {
    pub(crate) fn get_game_time(&self) -> GameTimeType {
        if self.sink.is_stopped() {
            return self.total_duration.as_secs_f64();
        }
        self.sink.get_pos().as_secs_f64() - 3.0
    }

    pub fn new(
        handle: OutputStreamHandle,
        song_info: &SongInfo,
        beatmap_file: SongBeatmapFile,
    ) -> anyhow::Result<Self> {
        let mut buf = vec![];
        let mut file = std::fs::File::open(&song_info.bgm_file)?;
        file.read_to_end(&mut buf)?;

        let buf = Cursor::new(buf);
        let decoder = Decoder::new(buf.clone())?;

        let samples = decoder;

        let total_duration = samples
            .total_duration()
            .ok_or(anyhow!("No audio duration"))?
            .add(Duration::from_secs_f32(3.0));
        if total_duration.as_secs() >= 2 * 60 * 60 {
            return Err(anyhow!("Cannot play audio with length >= 2h"));
        }

        let mut buffer_data =
            vec![0.0_f32; (samples.channels() as u32 * samples.sample_rate()) as usize * 3];
        let channels = samples.channels();
        let rate = samples.sample_rate();
        buffer_data.append(&mut samples.collect::<Vec<f32>>());

        let vol = STATIC_DATA
            .cfg_data
            .write()
            .map_err(|e| anyhow!("Cannot read lock for {:?}", e))?
            .get_f32_def("bgm_vol", 1.0);
        let mut sink =
            ControlledBufferHandle::new(&handle, SamplesBuffer::new(channels, rate, buffer_data))?;
        sink.set_volume(vol);

        let this = Self {
            total_duration,
            start_time: Instant::now(),
            hit_feedback: Default::default(),
            gaming: Box::new(Gaming::load_game(beatmap_file)),
            game_rect: Rect::ZERO,
            sink,
            score_display: Default::default(),
            end_remaining: None,
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
        if s.app.inputs.is_pressed(&[PhysicalKey::Code(KeyCode::Tab)]) {
            self.gaming.auto_play = !self.gaming.auto_play;
        }
        match &mut self.end_remaining {
            Some(x) => {
                *x -= s.dt;
                if (*x <= 0.0) {
                    trans = Trans::IntoSwitch;
                }
                self.sink
                    .set_volume(0.0_f32.max(self.sink.volume() - s.dt / 3.0))
            }
            None => {
                if self.gaming.is_end() {
                    self.end_remaining = Some(3.0);
                }
            }
        }

        // in fact, we do update in render, for we are polling.
        (trans, LoopState::POLL)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut trans = Trans::None;
        let game_time = self.get_game_time();
        if log::log_enabled!(target: "Gameplay", log::Level::Trace) {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            log::trace!(target: "Gameplay", "{} when {} (delta: {})", game_time, elapsed, elapsed - game_time);
        }
        let tick_sound_res: ResourceLocation = ResourceLocation::from_name("tick");
        self.gaming.tick(
            game_time,
            Some(|note: PlayingNoteType<'_>, result: NoteHitResult| {
                // auto play will play during tick.
                if result.is_miss() {
                    // The miss we should care.
                    self.hit_feedback.last_result = Some((result, Instant::now()));
                } else {
                    match note {
                        PlayingNoteType::Normal(_) => {
                            self.hit_feedback.last_result = Some((result, Instant::now()));
                            s.app.audio.as_mut().unwrap().play_sfx(&tick_sound_res);
                        }
                        PlayingNoteType::Long(note) => {
                            if note.start_result.is_none() {
                                s.app.audio.as_mut().unwrap().play_sfx(&tick_sound_res);
                            } else {
                                // we ignore the end result of long note.
                            }
                        }
                    }
                }
            }),
        );
        let gpu = s.app.gpu.as_mut().unwrap();
        let mut nr = s.app.world.fetch_mut::<NoteRenderer>();
        for (timing_group, x) in self.gaming.normal_notes.iter().enumerate() {
            let current_y = self.gaming.raw_file.timing_group.get_gameplay_y_game_time(
                game_time,
                timing_group as u8,
                self.gaming.ops.default_view_time,
            );
            let (normal_a, normal_b) = x.get_play_notes().as_slices();
            nr.collect_playing_notes(normal_a, gpu.get_screen_size_f32(), current_y);
            nr.collect_playing_notes(normal_b, gpu.get_screen_size_f32(), current_y);
        }
        for (timing_group, x) in self.gaming.long_notes.iter().enumerate() {
            let current_y = self.gaming.raw_file.timing_group.get_gameplay_y_game_time(
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
                
                let whole_rect = ui.available_rect_before_wrap();
                let game_rect = get_play_rect(whole_rect);
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    let score = self
                        .score_display
                        .mark_score(self.gaming.score_counter.get_score());
                    let score_str = format!("{:06}", score);
                    // monospace doesn't work
                    // ui.label(RichText::new(score_str).size(99.0).monospace());
                    for x in score_str.chars().rev() {
                        let used = ui.label(RichText::new(x).size(99.0)).rect.width();
                        let left = 61.0 - used;
                        if left > 0.0 {
                            ui.add_space(left);
                        }
                    }
                });

                // the center line
                ui.painter().hline(
                    ui.max_rect().x_range(),
                    game_rect.center().y,
                    Stroke::new(5.0, Color32::GRAY),
                );

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.no_select_text(
                        format!("{}", self.gaming.score_counter.get_combo()),
                        [300.0, 100.0],
                    );
                    if let Some(last_result) = self.hit_feedback.last_result {
                        if last_result.1.elapsed().as_secs_f32() <= 3.0 {
                            let elap = last_result.1.elapsed().as_secs_f32().min(1.0);
                            ui.no_select_text(
                                RichText::new(format!("{:?}", last_result.0.grade))
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
                    event,
                    is_synthetic,
                    ..
                } => {
                    if *is_synthetic || event.repeat {
                        return;
                    }
                    match event.physical_key {
                        PhysicalKey::Code(code) => match code {
                            _ => {
                                let input_game_time =
                                    self.get_game_time() - time.elapsed().as_secs_f64();

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
                                    if let Some((result, is_long)) = self
                                        .gaming
                                        .process_input(game_input, ((input_x + 0.75) * 4.0) as _)
                                    {
                                        self.hit_feedback.last_result =
                                            Some((result, Instant::now()));
                                        if !result.is_miss() {
                                            let tick_sound_res: ResourceLocation =
                                                ResourceLocation::from_name("tick");
                                            s.app.audio.as_mut().unwrap().play_sfx(&tick_sound_res);
                                        }
                                    }
                                } else {
                                    self.gaming.process_input_leave(
                                        game_input,
                                        ((input_x + 0.75) * 4.0) as _,
                                    );
                                }
                            }
                        },
                        PhysicalKey::Unidentified(_) => {}
                    }
                }
                WindowEvent::Resized(size) => self.update_game_region(*size),
                _ => {}
            },
            _ => {}
        }
    }

    fn switch(self: Box<Self>) -> Trans {
        Trans::Push(Box::new(EndResultState {
            result: BeatmapPlayResult::from_game(&self.gaming),
            gaming: self.gaming,
        }))
    }
}
