//! The real playing beatmap that contains detail notes.

pub mod file;
mod test;
mod play;

use std::cmp::Ordering;
use crate::game::beatmap::file::SongBeatmapFile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::game::OffsetType;

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

/// Game pos order by time, then x.
#[derive(Default, Copy, Clone)]
pub struct GamePos {
    pub x: f32,
    pub time: OffsetType,
}

impl GamePos {
    pub fn new(x: f32, time: OffsetType) -> Self {
        Self { x, time }
    }
}

impl PartialOrd for GamePos {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for GamePos {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
            .then(self.x.total_cmp(&other.x))
    }
}

impl PartialEq for GamePos {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for GamePos {}

#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct GameRect {
    min: GamePos,
    max: GamePos
}

impl GameRect {
    pub fn from_ab(a: GamePos, b: GamePos) -> Self {
        let l = a.x.min(b.x);
        let r = a.x.max(b.x);
        
        let bottom = a.time.min(b.time);
        let top = a.time.max(b.time);
        
        let min = GamePos::new(l, bottom);
        let max = GamePos::new(r, top);
        
        Self {
            min,
            max
        }
    }
}