//! The real playing beatmap that contains detail notes.

pub mod file;
mod test;

use crate::game::beatmap::file::SongBeatmapFile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum MapRule {
    Falling,
    FourKey,
}

pub const BEATMAP_EXT: &'static str = "rr";

#[derive(Clone, Debug)]
pub struct SongBeatmapInfo {
    pub file_path: PathBuf,
    pub song_beatmap_file: SongBeatmapFile,
}


impl Default for MapRule {
    fn default() -> Self {
        Self::Falling
    }
}

