use crate::game::beatmap::MapRule;
use crate::game::timing::TimingGroup;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatmapMetadata {
    pub title: String,
    pub artist: String,
    pub creator: String,
    pub version: String,
    pub source: String,
    pub tags: HashSet<String>,

}

impl BeatmapMetadata {
    pub fn new(title: String) -> Self {
        Self { title, ..Default::default() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongBeatmapFile {
    pub version: u8,
    pub metadata: BeatmapMetadata,
    pub timing_group: TimingGroup,
    pub rule: MapRule,

}

impl SongBeatmapFile {
    pub fn new(title: String) -> Self {
        Self { version: 0, metadata: BeatmapMetadata::new(title), timing_group: TimingGroup::new(), rule: MapRule::Falling }
    }
}





