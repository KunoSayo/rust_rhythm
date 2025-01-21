use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::{SongBeatmapInfo, BEATMAP_EXT};
use anyhow::anyhow;
use dashmap::DashMap;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use serde::Deserialize;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Debug)]
pub struct SongInfo {
    pub bgm_file: PathBuf,
    pub title: String,
    pub maps: Vec<SongBeatmapInfo>,
    /// Should we reload the maps
    pub dirty: AtomicBool,
}

///
pub struct SongManager {
    /// The songs dir
    pub root: PathBuf,
    pub songs: DashMap<String, Arc<SongInfo>>,
}

pub type SongManagerResourceType = Arc<SongManager>;

impl SongInfo {
    fn supported_bgm_format() -> &'static [&'static str] {
        &["mp3", "ogg"]
    }

    pub fn reload(&self) -> anyhow::Result<Self> {
        Self::load(self.bgm_file.parent().ok_or(anyhow!("No bgm file parent"))?)
    }

    pub fn load(song_dir_path: &Path) -> anyhow::Result<Self> {
        let title = song_dir_path.file_name().unwrap().to_string_lossy().to_string();

        let bgm_file = Self::supported_bgm_format().iter().filter_map(|ext| {
            let bgm_file = song_dir_path.join("bgm.".to_string() + ext);
            if bgm_file.exists() && bgm_file.is_file() {
                Some(bgm_file)
            } else {
                None
            }
        }).next();

        let bgm_file = if let Some(f) = bgm_file {
            f
        } else {
            return Err(anyhow!("No bgm found in {:?}", &song_dir_path))
        };


        let mut maps = std::fs::read_dir(&song_dir_path)?.par_bridge()
            .filter_map(|x: std::io::Result<DirEntry>| {
                match x {
                    Ok(entry) => {
                        if entry.path().is_file() && entry.path().extension().map(|x| x == BEATMAP_EXT).unwrap_or(false) {
                            Some(entry)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to read entry {:?}", e);
                        None
                    }
                }
            })
            .map(|entry| -> anyhow::Result<SongBeatmapInfo> {
                let data = std::fs::read(entry.path())?;
                let deserializer = &mut ron::Deserializer::from_bytes(&data[..])?;
                let beatmap = SongBeatmapFile::deserialize(deserializer)?;
                let info = SongBeatmapInfo {
                    file_path: entry.path(),
                    song_beatmap_file: beatmap,
                };
                Ok(info)
            })
            .filter_map(|result| {
                match result {
                    Ok(x) => Some(x),
                    Err(e) => {
                        log::warn!("Failed to parse beatmap info, caused by {:?}", e);
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        std::fs::read_dir(&song_dir_path)?;

        maps.sort_by(|x, y| x.song_beatmap_file.metadata.version
            .cmp(&y.song_beatmap_file.metadata.version));

        let song_info = SongInfo {
            bgm_file,
            title: title.clone(),
            maps,
            dirty: Default::default(),
        };

        Ok(song_info)
    }
}

impl SongManager {
    fn get_root() -> PathBuf {
        std::env::current_dir()
            .expect("Failed to get current dir")
            .join("songs")
    }


    pub fn init_manager() -> anyhow::Result<Self> {
        let root = Self::get_root();
        let _ = std::fs::create_dir_all(&root);
        let this = Self {
            root,
            songs: Default::default(),
        };


        // Reload songs
        std::fs::read_dir(&this.root)?.par_bridge()
            .filter_map(|x: std::io::Result<DirEntry>| {
                match x {
                    Ok(entry) => {
                        if entry.path().is_dir() && entry.path().file_name().is_some() {
                            Some(entry)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to read entry {:?}", e);
                        None
                    }
                }
            })
            .map(|x: DirEntry| -> anyhow::Result<()> {
                let song_dir_path = x.path();

                let song_info = SongInfo::load(&song_dir_path)?;

                this.songs.insert(song_info.title.clone(), song_info.into());

                Ok(())
            }).for_each(|x| {
            if let Err(e) = x {
                log::warn!("Failed to load song for {:?}", e);
            }
        });

        Ok(this)
    }

    pub fn load_new_info(&self, info: SongInfo) {
        self.songs.insert(info.title.clone(), info.into());
    }

    pub fn import_song(&self, song: &Path) -> anyhow::Result<Arc<SongInfo>> {
        let filename = song.file_name().ok_or(anyhow!("No filename"))?.to_string_lossy();

        let (filename_no_ext, ext) = match filename.split_once(".") {
            Some(x) => x,
            None => {
                return Err(anyhow!("Unsupported format for {}", filename));
            }
        };

        if !SongInfo::supported_bgm_format().iter().any(|x| ext == *x) {
            return Err(anyhow!("Unsupported format for {}", filename));
        }

        let song_dir = self.root.join(filename_no_ext);
        std::fs::create_dir_all(&song_dir)?;

        let bgm_file = song_dir.join("bgm.".to_string() + ext);

        std::fs::copy(song, &bgm_file)?;


        let info = SongInfo {
            bgm_file,
            title: filename_no_ext.to_string(),
            maps: vec![],
            dirty: AtomicBool::new(true),
        };

        let info = Arc::new(info);
        self.songs.insert(filename_no_ext.to_string(), info.clone());

        Ok(info)
    }
}


