//! The real playing beatmap that contains detail notes.

pub mod file;
pub mod play;
mod test;

use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::OffsetType;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

pub static FOUR_KEY_X: [f32; 4] = [-0.75, -0.25, 0.25, 0.75];

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
        self.time.cmp(&other.time).then(self.x.total_cmp(&other.x))
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
    max: GamePos,
}

impl GameRect {
    pub fn from_ab(a: GamePos, b: GamePos) -> Self {
        let l = a.x.min(b.x);
        let r = a.x.max(b.x);

        let bottom = a.time.min(b.time);
        let top = a.time.max(b.time);

        let min = GamePos::new(l, bottom);
        let max = GamePos::new(r, top);

        Self { min, max }
    }
}
