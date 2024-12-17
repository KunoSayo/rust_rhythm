use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use egui::{FontData, FontDefinitions, FontFamily};
use futures::executor::ThreadPool;
use log::info;
use once_cell::sync::Lazy;

use crate::engine::config::Config;
#[allow(unused)]
pub struct StaticData {
    pub font: FontDefinitions,
    pub cfg_data: RwLock<Config>,
}

pub static IO_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    ThreadPool::builder()
        .name_prefix("IO POOL")
        .before_stop(|x| {
            info!("IO Thread#{} stopping", x);
        })
        .stack_size(10 * 1024 * 1024)
        .pool_size(4)
        .create().expect("Create io thread pool failed")
});

#[allow(unused)]
pub static INITED: AtomicBool = AtomicBool::new(false);
#[allow(unused)]
pub static STATIC_DATA: Lazy<StaticData> = Lazy::new(|| {
    INITED.store(true, Ordering::Relaxed);
    info!("Loading lazy global data");
    let mut font = FontDefinitions::default();
    font.font_data.insert("cjk".into(), Arc::from(FontData::from_static(files::FONT_DATA)));
    font.families.get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "cjk".into());
    font.families.get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "cjk".into());
    let cfg_data = std::fs::read_to_string("cfg.toml").unwrap_or_else(|_| {
        if let Err(e) = std::fs::File::create("cfg.toml") {
            log::error!("Create config file failed for {:?}", e);
            panic!("{:?}", e);
        }
        "".into()
    });
    // Load config failed. Why not panic?
    let cfg_data = Config::load(&cfg_data).expect("Load config data failed");

    StaticData {
        font,
        cfg_data: RwLock::new(cfg_data),
    }
});

pub mod files {
    pub static FONT_DATA: &'static [u8] = include_bytes!("static_res/cjkFonts_allseto_v1.11.ttf");
}