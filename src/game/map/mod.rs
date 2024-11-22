//! The real playing map that contains detail notes.

mod file;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum MapRule {
    Falling
}

#[derive(Debug, Clone)]
pub struct SongMapInfo {
    pub maker: String,
    pub difficulty: String,
    pub map_file: PathBuf,
}


