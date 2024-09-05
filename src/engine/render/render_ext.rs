use wgpu::{Color, CommandEncoder, LoadOp, Operations, RenderPass, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureView};

#[allow(unused)]
pub trait CommandEncoderExt {
    /// Begin render pass with clear color.
    fn begin_clear_color<'a>(&'a mut self, color: &'a TextureView, clear_color: Color, store: StoreOp) -> RenderPass<'a>;

    /// Begin render pass with color and depth. will store both.
    fn begin_with_depth<'a>(&'a mut self, color: &'a TextureView, color_load: LoadOp<Color>,
                            depth: &'a TextureView, depth_load: LoadOp<f32>) -> RenderPass<'a>;


    /// Begin render pass with color and depth and stencil. will store color.
    fn begin_with_depth_stencil<'a>(&'a mut self, color: &'a TextureView, color_load: LoadOp<Color>,
                                    depth_stencil: &'a TextureView, depth_op: Operations<f32>,
                                    stencil_op: Operations<u32>) -> RenderPass<'a>;

    fn begin_multisample<'a>(&'a mut self, multi_sample_view: &'a TextureView, target: &'a TextureView, color_load: LoadOp<Color>,
                             depth: &'a TextureView, depth_load: LoadOp<f32>) -> RenderPass<'a>;
}

impl CommandEncoderExt for CommandEncoder {
    #[inline]
    fn begin_clear_color<'a>(&'a mut self, color: &'a TextureView, clear_color: Color, store: StoreOp) -> RenderPass<'a> {
        self.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color,
                resolve_target: None,
                ops: Operations { load: LoadOp::Clear(clear_color), store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
    #[inline]
    fn begin_with_depth<'a>(&'a mut self, color: &'a TextureView, color_load: LoadOp<Color>,
                            depth: &'a TextureView, depth_load: LoadOp<f32>) -> RenderPass<'a> {
        self.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color,
                resolve_target: None,
                ops: Operations {
                    load: color_load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(Operations { load: depth_load, store: StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    #[inline]
    fn begin_with_depth_stencil<'a>(&'a mut self, color: &'a TextureView, color_load: LoadOp<Color>,
                                    depth_stencil: &'a TextureView, depth_op: Operations<f32>,
                                    stencil_op: Operations<u32>) -> RenderPass<'a> {
        self.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color,
                resolve_target: None,
                ops: Operations {
                    load: color_load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_stencil,
                depth_ops: Some(depth_op),
                stencil_ops: Some(stencil_op),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
    #[inline]
    fn begin_multisample<'a>(&'a mut self, multi_sample_view: &'a TextureView, target: &'a TextureView, color_load: LoadOp<Color>,
                             depth: &'a TextureView, depth_load: LoadOp<f32>) -> RenderPass<'a> {
        self.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: multi_sample_view,
                resolve_target: Some(target),
                ops: Operations {
                    load: color_load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(Operations { load: depth_load, store: StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}

