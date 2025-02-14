use std::path::PathBuf;
use std::sync::Arc;
use anyhow::anyhow;
use dashmap::DashMap;
use log::info;
use wgpu::{Device, Queue};

use crate::engine::{ResourceLocation, TextureWrapper};
use crate::engine::atlas::TextureAtlas;

#[derive(Debug)]
pub struct ResourcePack {
    pub root_dir: PathBuf,
}

impl ResourcePack {
    fn builtin() -> anyhow::Result<Self> {
        let app_root = std::env::current_dir()?;
        let res_root = if app_root.join("../res").exists() { app_root.join("../res") } else { app_root };
        info!("Builtin resource pack path is {:?}", res_root);
        Ok(Self {
            root_dir: res_root,
        })
    }


    pub fn load_asset(&self, path: &str) -> Option<std::io::Result<Vec<u8>>> {
        let path = self.root_dir.join("assets").join(path);
        if let Ok(true) = path.try_exists() {
            Some(std::fs::read(path))
        } else {
            None
        }
    }
}


#[allow(unused)]
#[derive(Debug)]
pub struct ResourceManager {
    builtin: ResourcePack,
    /// Index 0 will be check first
    packs: Vec<ResourcePack>,
    // pub fonts: DashMap<String, FontArc>,
    pub textures: DashMap<String, TextureWrapper>,
    pub atlas: DashMap<ResourceLocation, Arc<TextureAtlas>>,
}

#[allow(unused)]
impl ResourceManager {
    pub fn new() -> anyhow::Result<Self> {
        let builtin_pack = ResourcePack::builtin()?;
        Ok(Self {
            builtin: builtin_pack,
            packs: vec![],
            // fonts: Default::default(),
            textures: Default::default(),
            atlas: Default::default(),
        })
    }


    /// Load the asset from packs. no cache
    pub fn load_asset(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        for pack in &self.packs {
            if let Some(r) = pack.load_asset(path) {
                return Ok(r?);
            }
        }


        if let Some(r) = self.builtin.load_asset(path) {
            return Ok(r?);
        }

        Err(anyhow!("The path {:?} is not valid", path))
    }

    pub fn load_texture(&self, device: &Device, queue: &Queue, key: String, path: &str) -> anyhow::Result<()> {
        info!("Loading texture {} in {}", &key, path);
        let img_data = self.load_asset(path)?;
        let texture = TextureWrapper::from_bytes(device, queue, &img_data, Some(&key), false)?;
        self.textures.insert(key, texture);
        Ok(())
    }

    pub async fn load_texture_async(&self, device: &Device, queue: &Queue, key: String, path: &str) -> anyhow::Result<()> {
        self.load_texture(device, queue, key, path)
    }
}
