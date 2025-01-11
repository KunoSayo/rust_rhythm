//! The real playing beatmap that contains detail notes.

pub mod file;

use crate::game::beatmap::file::SongBeatmapFile;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum MapRule {
    Falling
}

pub const BEATMAP_EXT: &'static str = "rr";

#[derive(Debug, Clone)]
pub struct SongBeatmapInfo {
    pub song_beatmap_file: SongBeatmapFile
}


