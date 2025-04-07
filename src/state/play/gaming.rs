use std::io::{Cursor, Read};
use std::ops::Add;
use crate::engine::{GameState, LoopState, StateData};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::play::Gaming;
use rodio::static_buffer::StaticSamplesBuffer;
use rodio::{Decoder, OutputStreamHandle, Sink, Source};
use std::time::Duration;
use anyhow::anyhow;
use rodio::buffer::SamplesBuffer;
use tokio::time::Instant;
use wgpu::naga::Statement::Loop;
use crate::engine::global::STATIC_DATA;
use crate::game::song::SongInfo;

pub struct GamingState {
    pub total_duration: Duration,
    pub start_time: Instant,
    gaming: Gaming,
    sink: Sink,
}
impl GamingState {
    pub fn new(handle: OutputStreamHandle, song_info: &SongInfo, beatmap_file: SongBeatmapFile) -> anyhow::Result<Self> {
        let sink = Sink::try_new(&handle)?;
        sink.append(SamplesBuffer::new(2, 1, &[0i16, 0, 0, 0, 0, 0]));
        sink.pause();
        sink.try_seek(Duration::ZERO).expect("?");

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
            sink,
        };
        Ok(this)
    }
}

impl GameState for GamingState {
    fn start(&mut self, _: &mut StateData) -> LoopState {
        log::info!("Gaming state start!");
        self.sink.play();
        self.start_time = Instant::now();
        LoopState::POLL
    }
}
