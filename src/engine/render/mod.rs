use std::any::type_name;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use once_cell::sync::Lazy;
use wgpu::*;

pub use texture::*;

use crate::engine::{ResourceManager, TextureInfo, TextureWrapper, WgpuData};

pub mod invert_color;
pub mod point;
pub mod texture;
pub mod state;
pub mod render_ext;
pub mod renderer;
pub mod renderer3d;
pub mod uniform;
pub mod camera;

static INSTANCE: Lazy<Instance> = Lazy::new(|| Instance::new(InstanceDescriptor::default()));

pub trait Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a>;
}


#[derive(Debug)]
pub struct MainRenderViews {
    buffers: [TextureWrapper; 2],
    depth: TextureWrapper,
    extra: HashMap<String, TextureWrapper>,
    main: usize,
}


pub struct MainRendererData {
    pub staging_belt: util::StagingBelt,
    pub egui_rpass: egui_wgpu::Renderer,
}

impl Debug for MainRendererData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<Self>())
            .finish()
    }
}

impl MainRendererData {
    pub fn new(gpu: &WgpuData, _handles: &ResourceManager) -> Self {
        let staging_belt = util::StagingBelt::new(2048);
        let egui_rpass = egui_wgpu::Renderer::new(&gpu.device, gpu.surface_cfg.format, None, 1);
        Self {
            staging_belt,
            egui_rpass,
        }
    }
}


#[allow(unused)]
impl MainRenderViews {
    pub fn new(device: &Device, surface_cfg: &SurfaceConfiguration) -> Self {
        let size = (surface_cfg.width, surface_cfg.height);
        let texture_desc = TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: surface_cfg.format,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[surface_cfg.format],
        };

        let buffer_a = {
            let texture = device.create_texture(&texture_desc);
            let view = texture.create_view(&TextureViewDescriptor::default());

            TextureWrapper {
                texture,
                view,
                info: TextureInfo::new(size.0, size.1),
            }
        };

        let buffer_b = {
            let texture = device.create_texture(&texture_desc);
            let view = texture.create_view(&TextureViewDescriptor::default());

            TextureWrapper {
                texture,
                view,
                info: TextureInfo::new(size.0, size.1),
            }
        };

        let depth = TextureWrapper::create_depth_texture(device, surface_cfg, "Main Depth Texture");

        Self {
            buffers: [buffer_a, buffer_b],
            depth,
            extra: Default::default(),
            main: 0,
        }
    }

    /// Get the buffer that will present to window.
    pub fn get_screen(&self) -> &TextureWrapper {
        &self.buffers[self.main]
    }

    /// Get the buffer with same size as screen but won't present to window.
    pub fn get_off_screen(&self) -> &TextureWrapper {
        &self.buffers[self.main ^ 1]
    }

    /// if not present then create
    pub fn check_extra_with_size(&mut self, id: &str, device: &Device, size: (u32, u32), format: TextureFormat) {
        {
            if let Some(extra) = self.extra.get(id) {
                if extra.info.width == size.0 && extra.info.height == size.1 {
                    return;
                }
            }
        }
        let wrapper = TextureWrapper::new_with_size(device, format, size);
        self.extra.insert(id.into(), wrapper);
    }

    pub fn get_extra(&self, id: &str) -> Option<&TextureWrapper> {
        self.extra.get(id)
    }

    pub fn get_depth_view(&self) -> &TextureWrapper {
        &self.depth
    }

    /// Return (src, dst)
    #[allow(unused)]
    pub fn swap_screen(&mut self) -> (&TextureWrapper, &TextureWrapper) {
        let src = self.main;
        self.main = (self.main + 1) & 1;
        let dst = self.main;
        (&self.buffers[src], &self.buffers[dst])
    }
}
