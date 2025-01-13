use crate::engine::global::STATIC_DATA;
use crate::engine::{GameState, LoopState, StateData, Trans};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::SongBeatmapInfo;
use crate::game::song::SongInfo;
use anyhow::anyhow;
use egui::panel::TopBottomSide;
use egui::{Context, Label};
use rodio::{Decoder, OutputStreamHandle, Sink, Source};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub struct BeatMapEditor {
    pub song_info: Arc<SongInfo>,
    pub song_beatmap_file: SongBeatmapFile,
    save_path: Option<PathBuf>,
    total_duration: Duration,
    sink: Sink,
}

impl BeatMapEditor {
    pub fn new(song_info: Arc<SongInfo>, handle: OutputStreamHandle) -> anyhow::Result<Self> {
        Self::with_file(song_info, None, handle)
    }

    pub fn with_file(song_info: Arc<SongInfo>, info: Option<SongBeatmapInfo>, s: OutputStreamHandle) -> anyhow::Result<Self> {
        let sink = Sink::try_new(&s)
            .expect("Failed to new sink");

        let file = std::fs::File::open(&song_info.bgm_file)?;

        let decoder = Decoder::new(file)?;
        let samples = decoder.convert_samples::<f32>();
        let total_duration = samples.total_duration().ok_or(anyhow!("No audio duration"))?;
        sink.pause();
        sink.append(samples);

        let vol = STATIC_DATA.cfg_data.write()
            .map_err(|e| anyhow!("Cannot read lock for {:?}", e))?
            .get_f32_def("bgm_vol", 1.0);
        sink.set_volume(vol);

        let path = info.as_ref().map(|x| x.file_path.clone());
        Ok(Self {
            song_beatmap_file: info.map(|x| x.song_beatmap_file).unwrap_or(SongBeatmapFile::new(song_info.title.clone())),
            song_info,
            sink,
            save_path: path,
            total_duration,
        })
    }
}

impl BeatMapEditor {
    fn get_progress(&self) -> Duration {
        let dur = self.sink.get_pos();
        dur.mul_f32(self.sink.speed())
    }
}
impl GameState for BeatMapEditor {
    fn start(&mut self, s: &mut StateData) {}

    fn update(&mut self, _: &mut StateData) -> (Trans, LoopState) {
        let mut tran = Trans::None;

        let mut loop_state = LoopState::WAIT;

        if !self.sink.is_paused() {
            loop_state = LoopState::POLL;
        }

        (tran, loop_state)
    }


    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut tran = Trans::None;

        egui::TopBottomPanel::new(TopBottomSide::Bottom, "audio")
            .min_height(100.0)
            .show(ctx, |ui| {
                Label::new(format!(""))
            });


        tran
    }

    fn stop(&mut self, s: &mut StateData) {}
}