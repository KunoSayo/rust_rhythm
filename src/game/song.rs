use std::path::PathBuf;
use crate::game::map::SongMapInfo;

#[derive(Clone, Debug)]
pub struct SongInfo {
    pub bgm_file: PathBuf,
    pub title: String,
    pub author: String,
    pub tags: Vec<String>,
    pub maps: Vec<SongMapInfo>,
}

#[derive(Default)]
pub struct SongManager {
    songs: Vec<SongInfo>
}

