use wgpu::util::RenderEncoder;

use crate::engine::WgpuData;

/// Renderer for the type `Obj`
pub trait Renderer<Obj>: Send + 'static {
    fn render<'a, T: RenderEncoder<'a>>(&'a mut self, encoder: &mut T, state: &WgpuData, objs: &'a [Obj]);
}

