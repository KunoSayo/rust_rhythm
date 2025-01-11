use std::sync::Arc;
use crate::engine::GameState;
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::song::SongInfo;

pub struct BeatMapEditor {
    pub song_info: Arc<SongInfo>,
    pub song_beatmap_file: SongBeatmapFile
}

impl BeatMapEditor {
    pub fn new(song_info: Arc<SongInfo>) -> Self {
        Self {
            song_beatmap_file: SongBeatmapFile::new(song_info.title.clone()),
            song_info,
        }
    }
}

impl GameState for BeatMapEditor {
    
}