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
pub struct StaticRendererData {
    pub nearest_sampler: Arc<Sampler>,
    pub rect_index_buffer: Arc<Buffer>,
}

impl StaticRendererData {
    pub fn new(device: &Device) -> Self {
        let nearest_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            lod_max_clamp: 0.0,
            ..Default::default()
        }).into();

        // We have 8192 rects
        let rect_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&(0u16..8192u16 * 6).map(|x| x / 6 * 4 + [0, 1, 2, 1, 2, 3][(x % 6) as usize]).collect::<Vec<u16>>()),
            usage: BufferUsages::INDEX,
        }).into();
        Self { nearest_sampler, rect_index_buffer }
    }
}