use std::sync::Arc;

use egui::{Context, Style, ViewportId};
use egui_winit::State;
use log::{info, warn};
use specs::{World, WorldExt};
use winit::window::Window;

use crate::engine::manager::{EngineEventLoopProxy, EventLoopTargetType};
use crate::engine::{AudioData, BakedInputs, MainRendererData, ResourceManager, WgpuData};

pub struct AppInstance {
    pub window: Window,
    pub gpu: Option<WgpuData>,
    pub render: Option<MainRendererData>,
    pub res: Arc<ResourceManager>,
    pub last_render_time: std::time::Instant,
    pub last_update_time: std::time::Instant,
    pub egui: State,
    pub egui_ctx: Context,

    pub inputs: BakedInputs,
    pub world: World,

    pub audio: Option<AudioData>,
}

impl AppInstance {
    fn new_with_gpu(window: Window, gpu: Option<WgpuData>) -> anyhow::Result<Self> {
        let res = ResourceManager::new()?;
        let render = match &gpu { Some(gpu) => {
            Some(MainRendererData::new(gpu, &res))
        } _ => {
            None
        }};
        info!("Got the lua");
        let egui_ctx = Context::default();
        info!("Got the egui context");
        let mut style = Style::default();
        style.clone_from(&egui_ctx.style());
        for (_, s) in &mut style.text_styles {
            s.size *= 1.25;
        }
        egui_ctx.set_style(style);
        if gpu.is_some() {
            // egui_ctx.set_pixels_per_point(window.scale_factor() as f32);
            // info!("Set the egui context scale factor to {}", window.scale_factor());
        }

        let egui = State::new(
            egui_ctx.clone(),
            ViewportId::ROOT,
            &window,
            egui_ctx.native_pixels_per_point(),
            None,
            None,
        );

        let al = std::panic::catch_unwind(|| match AudioData::new() {
            Ok(al) => Some(al),
            Err(e) => {
                warn!("Load audio failed for {:?}", e);
                None
            }
        })
        .unwrap_or_else(|e| {
            warn!(
                "Get audio even panicked for {:?} with type id {:?}",
                e,
                e.type_id()
            );
            None
        });

        info!("Creating thread pool");

        info!("Almost got all window instance field");
        Ok(Self {
            window,
            gpu,
            render,
            res: res.into(),
            last_render_time: std::time::Instant::now(),
            last_update_time: std::time::Instant::now(),
            egui,
            egui_ctx,
            inputs: Default::default(),
            world: World::new(),
            audio: al,
        })
    }

    /// Create the app instance with the same gpu data
    #[inline]
    pub fn create_from_gpu(window: Window, gpu: &WgpuData) -> anyhow::Result<Self> {
        let gpu = WgpuData::create_from_exists(&window, gpu).ok();
        Self::new_with_gpu(window, gpu)
    }

    #[inline]
    pub fn new(window: Window, el: &impl EngineEventLoopProxy) -> anyhow::Result<Self> {
        let gpu = WgpuData::new(&window, el).ok();
        Self::new_with_gpu(window, gpu)
    }
}
