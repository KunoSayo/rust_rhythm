use crate::game::beatmap::MapRule;
use crate::game::note::{LongNote, NormalNote};
use crate::game::timing::{get_ron_options, get_ron_options_for_implicit_some, TimingGroup};
use anyhow::anyhow;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io;
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
        Self {
            title,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongBeatmapFile {
    pub version: u8,
    pub metadata: BeatmapMetadata,
    pub timing_group: TimingGroup,
    #[serde(default)]
    pub normal_notes: Vec<NormalNote>,
    #[serde(default)]
    pub long_notes: Vec<LongNote>,
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
        ser_to_ron(&self, file, Some(PrettyConfig::default()))?;

        Ok(())
    }

    pub fn new(title: String) -> Self {
        Self {
            version: 0,
            metadata: BeatmapMetadata::new(title),
            timing_group: TimingGroup::new(),
            normal_notes: vec![],
            long_notes: vec![],
            rule: MapRule::Falling,
        }
    }
    
    /// Update the data and cache.
    pub fn update(&mut self) {
        self.timing_group.update();
    }
}

pub fn ser_to_ron<T: Serialize>(
    data: &T,
    writer: impl io::Write,
    cfg: Option<PrettyConfig>,
) -> anyhow::Result<()> {
    let mut s = ron::Serializer::with_options(writer, cfg, get_ron_options())?;
    data.serialize(&mut s)?;
    Ok(())
}

pub fn de_from_ron<'a, T: Deserialize<'a>>(data: &'a [u8]) -> anyhow::Result<T> {
    let err;
    match ron::Deserializer::from_bytes_with_options(data, get_ron_options()) {
        Ok(ref mut der) => match T::deserialize(der) {
            Ok(result) => return Ok(result),
            Err(e) => {
                err = Some(anyhow!(e));
            }
        },
        Err(e) => {
            err = Some(anyhow!(e));
        }
    }
    if let Ok(ref mut der) =
        ron::Deserializer::from_bytes_with_options(data, get_ron_options_for_implicit_some())
    {
        if let Ok(v) = T::deserialize(der) {
            return Ok(v);
        }
    }
    if let Ok(ref mut der) = ron::Deserializer::from_bytes(data) {
        if let Ok(v) = T::deserialize(der) {
            return Ok(v);
        }
    }
    Err(err.unwrap())
}
