use bytemuck::Pod;
use bytemuck::Zeroable;
use wgpu::{include_wgsl, BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, LoadOp, Operations, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, StoreOp, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat};

use crate::engine::app::AppInstance;
use crate::engine::WgpuData;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Pod, Zeroable)]
#[repr(C, align(4))]
pub struct InvertColorVertexData {
    pub pos: [f32; 2],
}

const VERTEX_DATA_SIZE: usize = std::mem::size_of::<InvertColorVertexData>();

#[allow(unused)]
#[derive(Clone, Copy, Debug)]
pub struct InvertColorCircle {
    pub center: [f32; 2],
    pub radius: f32,
}

#[allow(unused)]
#[derive(Debug)]
pub struct InvertColorRenderer {
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
}

#[allow(unused)]
impl InvertColorRenderer {
    pub fn new(state: &WgpuData) -> Self {
        let texture_format = state.surface_cfg.format;
        let device = &state.device;
        //done bind group
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<InvertColorVertexData>() as u64 * 16 * 4),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let wgsl = include_wgsl!("invert_color.wgsl");
        let shader = device.create_shader_module(wgsl);


        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: VERTEX_DATA_SIZE as u64,
                    step_mode: Default::default(),
                    attributes: &[VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Subtract,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
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

    pub fn render<'a>(&'a self, window: &AppInstance, render_target: &TextureView, circles: &[InvertColorCircle]) {
        let gpu = if let Some(state) = &window.gpu { state } else { return; };

        profiling::scope!("Invert Color new encoder");
        let rp_attach = [Some(RenderPassColorAttachment {
            view: render_target,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        })];
        {
            let mut data = Vec::with_capacity(VERTEX_DATA_SIZE * 16 * 4);
            let to_normal = |obj: &InvertColorCircle, i| {
                // 0 1
                // 2 3
                let x = if i & 1 == 0 {
                    obj.center[0] - obj.radius
                } else {
                    obj.center[0] + obj.radius
                };
                let y = if i < 2 {
                    obj.center[1] - obj.radius
                } else {
                    obj.center[1] + obj.radius
                };
                //    +y
                // -x O +x
                //    -y
                let x = (2.0 * x / gpu.surface_cfg.width as f32) - 1.0;
                let y = (-2.0 * y / gpu.surface_cfg.height as f32) + 1.0;
                [x, y]
            };
            for x in circles.chunks(16) {
                data.clear();
                let render_len = x.iter().filter(|x| x.radius > 0.0).inspect(|x| {
                    for i in 0..4 {
                        let pos = to_normal(x, i);
                        data.extend_from_slice(bytemuck::cast_slice(&pos[..]));
                    }
                }).count();
                gpu.queue.write_buffer(&self.vertex_buffer, 0, &data[..]);
                gpu.queue.submit(None);
                let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Invert Color Encoder") });
                let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("ic rp"),
                    color_attachments: &rp_attach,
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rp.set_pipeline(&self.render_pipeline);
                rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                for i in 0..render_len {
                    let i = i as u32;
                    rp.draw(i * 4..4 + i * 4, 0..1);
                }
                drop(rp);
                gpu.queue.submit(Some(encoder.finish()));
            }
        }
    }
}