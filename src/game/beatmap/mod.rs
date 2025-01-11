//! The real playing beatmap that contains detail notes.

pub mod file;

use std::path::PathBuf;
use crate::game::beatmap::file::SongBeatmapFile;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum MapRule {
    Falling
}

pub const BEATMAP_EXT: &'static str = "rr";

#[derive(Debug)]
pub struct SongBeatmapInfo {
    pub file_path: PathBuf,
    pub song_beatmap_file: SongBeatmapFile
}


