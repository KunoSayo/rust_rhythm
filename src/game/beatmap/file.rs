use crate::game::beatmap::MapRule;
use crate::game::note::{LongNote, NormalNote};
use crate::game::timing::TimingGroup;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatmapMetadata {
    pub title: String,
    pub artist: String,
    pub creator: String,
    pub version: String,
    pub source: String,
    // Comma split tags
    pub tags: String,

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
    #[serde(default)]
    pub normal_note: Vec<NormalNote>,
    #[serde(default)]
    pub long_note: Vec<LongNote>,
    #[serde(default)]
    pub rule: MapRule,
}


impl SongBeatmapFile {
    pub(crate) fn get_show_name(&self) -> String {
        format!("{}[{}]", self.metadata.title, self.metadata.version)
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        let file = std::fs::File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let mut s = ron::Serializer::new(file, Some(PrettyConfig::default()))?;
        self.serialize(&mut s)?;


        Ok(())
    }

    pub fn new(title: String) -> Self {
        Self {
            version: 0,
            metadata: BeatmapMetadata::new(title),
            timing_group: TimingGroup::new(),
            normal_note: vec![],
            long_note: vec![],
            rule: MapRule::Falling,
        }
    }
}





