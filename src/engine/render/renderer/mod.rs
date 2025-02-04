pub mod texture_renderer;

use std::sync::Arc;
use wgpu::util::{BufferInitDescriptor, DeviceExt, RenderEncoder};
use wgpu::{Buffer, Device, Sampler, SamplerDescriptor};

use crate::engine::prelude::{AddressMode, BufferUsages, FilterMode};
use crate::engine::WgpuData;

/// Renderer for the type `Obj`
pub trait Renderer<Obj>: Send + 'static {
    fn render<'a, T: RenderEncoder<'a>>(&'a mut self, encoder: &mut T, state: &WgpuData, objs: &'a [Obj]);
}

#[derive(Debug, Clone)]
pub struct MainRendererData {
    pub nearest_sampler: Arc<Sampler>,
    pub rect_index_buffer: Arc<Buffer>,
}

impl MainRendererData {
    pub fn new(device: &Device) -> Self {
        let nearest_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            lod_max_clamp: 0.0,
            ..Default::default()
        }).into();

        // We have 42 rects
        let rect_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &(0u8..252).map(|x| x / 6 * 4 + [0, 1, 2, 1, 2, 3][(x % 6) as usize]).collect::<Vec<_>>(),
            usage: BufferUsages::INDEX,
        }).into();
        Self { nearest_sampler, rect_index_buffer }
    }
}