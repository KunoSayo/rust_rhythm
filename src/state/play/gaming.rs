use std::time::Duration;
use rodio::Sink;
use crate::game::beatmap::file::SongBeatmapFile;



pub struct BeatMapPlayer {
    pub beatmap: SongBeatmapFile,
    pub total_duration: Duration,
    sink: Sink,
}
