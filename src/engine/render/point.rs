use bytemuck::Pod;
use bytemuck::Zeroable;
use wgpu::{include_wgsl, BlendState, Buffer, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, LoadOp, Operations, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, StoreOp, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat};

use crate::engine::app::AppInstance;
use crate::engine::WgpuData;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Pod, Zeroable)]
#[repr(C, align(4))]
pub struct PointVertexData {
    pub color: [f32; 4],
    pub pos: [f32; 2],
}

const VERTEX_DATA_SIZE: usize = std::mem::size_of::<PointVertexData>();
const OBJ_VERTEX_COUNT: usize = 4;

#[allow(unused)]
#[derive(Debug)]
pub struct PointRenderer {
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
}

#[allow(unused)]
impl PointRenderer {
    pub fn new(state: &WgpuData) -> Self {
        let texture_format = state.surface_cfg.format;
        let device = &state.device;
        //done bind group
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: (VERTEX_DATA_SIZE * OBJ_VERTEX_COUNT * 16) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let wgsl = include_wgsl!("point.wgsl");
        let shader = device.create_shader_module(wgsl);


        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: VERTEX_DATA_SIZE as u64,
                    step_mode: Default::default(),
                    attributes: &[VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    }, VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 4 * 4,
                        shader_location: 1,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            vertex_buffer,
        }
    }

    pub fn render<'a>(&'a self, window: &AppInstance, render_target: &TextureView, points: &[PointVertexData]) {
        let gpu = if let Some(state) = &window.gpu { state } else { return; };
        profiling::scope!("Point Renderer");
        let rp_attach = [Some(RenderPassColorAttachment {
            view: render_target,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        })];
        {
            let mut data = Vec::with_capacity(VERTEX_DATA_SIZE * 16 * OBJ_VERTEX_COUNT);
            let to_normal = |obj: &PointVertexData, i| {
                let radius = 3.0;
                // 0 1
                // 2 3
                let x = if i & 1 == 0 {
                    obj.pos[0] - radius
                } else {
                    obj.pos[0] + radius
                };
                let y = if i < 2 {
                    obj.pos[1] + radius
                } else {
                    obj.pos[1] - radius
                };
                //    +y
                // -x O +x
                //    -y
                let x = (2.0 * x / gpu.surface_cfg.width as f32) - 1.0;
                let y = (-2.0 * y / gpu.surface_cfg.height as f32) + 1.0;
                [x, y]
            };
            for x in points.chunks(16) {
                data.clear();
                let render_len = x.iter().inspect(|x| {
                    for i in 0..4 {
                        let pos = to_normal(x, i);
                        data.extend_from_slice(bytemuck::cast_slice(&x.color[..]));
                        data.extend_from_slice(bytemuck::cast_slice(&pos[..]));
                    }
                }).count();
                gpu.queue.write_buffer(&self.vertex_buffer, 0, &data[..]);
                gpu.queue.submit(None);
                let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Pointer Render Encoder") });
                let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("p rp"),
                    color_attachments: &rp_attach,
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rp.set_pipeline(&self.render_pipeline);
                rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                for i in 0..render_len {
                    rp.draw(i as u32 * 4..i as u32 * 4 + 4, 0..1);
                }
                drop(rp);
                gpu.queue.submit(Some(encoder.finish()));
            }
        }
    }
}