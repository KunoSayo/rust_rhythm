use crate::game::beatmap::MapRule;
use crate::game::timing::TimingGroup;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongBeatmapFile {
    pub version: u8,
    pub timing_group: TimingGroup,
    pub rule: MapRule,
}


