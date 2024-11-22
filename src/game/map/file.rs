use crate::game::map::MapRule;
use crate::game::timing::TimingGroup;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongMapFile {
    pub version: u8,
    pub timing_group: TimingGroup,
    pub rule: MapRule,
}


