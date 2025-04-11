use crate::engine::global::STATIC_DATA;
use crate::engine::renderer::texture_renderer::TextureRenderer;
use crate::engine::{GameState, LoopState, StateData, StateEvent, Trans};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::play::Gaming;
use crate::game::get_play_rect;
use crate::game::render::NoteRenderer;
use crate::game::song::SongInfo;
use anyhow::anyhow;
use egui::{Color32, Context, Frame, Pos2, Rect, Stroke};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStreamHandle, Sink, Source};
use std::io::{Cursor, Read};
use std::ops::{Add, Deref};
use std::time::Duration;
use tokio::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::keyboard::{Key, KeyCode, PhysicalKey};

pub struct GamingState {
    pub total_duration: Duration,
    pub start_time: Instant,
    gaming: Gaming,
    game_rect: Rect,
    sink: Sink,
}
impl GamingState {
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
            trans = Trans::Exit;
        }
        // in fact, we do update in render, for we are polling.
        (trans, LoopState::POLL)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut trans = Trans::None;
        let game_time = self.sink.get_pos().as_secs_f32() - 3.0;
        self.gaming.tick(game_time);
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
                ui.painter().hline(self.game_rect.x_range(), self.game_rect.center().y, Stroke::new(1.0, Color32::WHITE));
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
                        _ => {}
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
