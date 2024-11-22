use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct SongInfo {
    pub bgm_file: PathBuf,
    pub title: String,
    pub author: String,
    pub tags: Vec<String>,
}

pub struct SongManager {}