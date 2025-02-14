use crate::engine::prelude::*;
use crate::engine::render_ext::CommandEncoderExt;
use crate::engine::renderer::StaticRendererData;
use crate::engine::uniform::uniform_bind_buffer_layout_entry;
use bytemuck::{Pod, Zeroable};
use nalgebra::{Vector2, Vector4};
use std::mem::size_of;
use std::num::NonZeroU64;
use wgpu::util::{RenderEncoder, StagingBelt};

#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone, Debug)]
pub struct TextureObjectVertex {
    position: Vector2<f32>,
    tex_coords: Vector2<f32>,
}

#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone, Debug)]
pub struct TextureObject {
    // 48 * 2 Bytes
    position: [Vector2<f32>; 6],
    tex_coords: [Vector2<f32>; 6],
}

impl TextureObject {
    pub fn new_rect(left_up: Vector2<f32>, right_bottom: Vector2<f32>, tex: &[Vector2<f32>; 4]) -> Self {
        let right_up = Vector2::new(right_bottom.x, left_up.y);
        let left_bottom = Vector2::new(left_up.x, right_bottom.y);
        Self {
            position: [left_up, right_up, left_bottom, left_bottom, right_up, right_bottom],
            tex_coords: [tex[0], tex[1], tex[2], tex[2], tex[1], tex[3]],
        }
    }
}


#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone)]
pub struct FragUniform {
    pub tint: Vector4<f32>,
}

impl Vertex for TextureObjectVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        const LAYOUT: VertexBufferLayout = VertexBufferLayout {
            array_stride: size_of::<TextureObjectVertex>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x2],
        };
        LAYOUT
    }
}


// group 0 for base layout: camera sampler light
// group 1 for planes using the same texture
pub struct TextureRenderer {
    /// Group0.
    pub base_bind_layout: BindGroupLayout,
    /// Group1.
    /// Bindings 0: texture view
    pub obj_layout: BindGroupLayout,
    pub light_uniform: Buffer,
    pub bind_group_zero: BindGroup,
    pub normal_rp: RenderPipeline,
    pub stream_draw_vertex_buffer: Buffer,
}


impl TextureRenderer {
    pub fn new(gpu: &WgpuData, shader: &ShaderModule) -> Self {
        let device = &gpu.device;
        let base_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Renderer base bind group"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                count: None,
            }],
        });


        let base_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &base_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Sampler(&gpu.data.nearest_sampler),
            }],
        });

        let obj_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: Default::default(),
                    view_dimension: Default::default(),
                    multisampled: false,
                },
                count: None,
            }, uniform_bind_buffer_layout_entry(1,
                                                ShaderStages::FRAGMENT,
                                                size_of::<FragUniform>() as u64)],
        });


        let frag_uniform = device.create_buffer(&BufferDescriptor {
            label: None,
            size: size_of::<FragUniform>() as _,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });


        let rp_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&base_bind_layout, &obj_layout],
            push_constant_ranges: &[],
        });
        let targets = [Some(ColorTargetState {
            format: gpu.surface_cfg.format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];
        let rpd = RenderPipelineDescriptor {
            label: None,
            layout: Some(&rp_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[TextureObjectVertex::desc()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &targets,
            }),
            multiview: None,
            cache: None,
        };
        let normal_rp = device.create_render_pipeline(&rpd);

        Self {
            base_bind_layout,
            obj_layout,
            light_uniform: frag_uniform,
            bind_group_zero: base_bind_group,
            normal_rp,
            stream_draw_vertex_buffer: device.create_buffer(&BufferDescriptor {
                label: Some("Texture renderer buffer"),
                size: (size_of::<TextureObject>() * 512) as BufferAddress,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }
}

impl TextureRenderer {
    fn render<'a>(&'a mut self, device: &Device, encoder: &mut CommandEncoder, srd: &StaticRendererData, texture_bind: &BindGroup, target: &'a TextureView, staging_belt: &mut StagingBelt, objs: &'a [TextureObject]) {
        let mut rp = encoder.begin_normal(target)
            .forget_lifetime();
        rp.set_pipeline(&self.normal_rp);
        rp.set_bind_group(0, &self.bind_group_zero, &[]);
        rp.set_bind_group(1, texture_bind, &[]);
        rp.set_index_buffer(srd.rect_index_buffer.slice(..), IndexFormat::Uint16);
        for x in objs.chunks(256) {
            let size = x.len() * size_of::<TextureObject>();

            let mut buffer = staging_belt
                .write_buffer(encoder, &self.stream_draw_vertex_buffer, 0, NonZeroU64::new(size as u64).unwrap(), device);
            buffer[..size].copy_from_slice(bytemuck::cast_slice(x));
            drop(buffer);

            rp.draw_indexed(0..(x.len() * 6) as u32, 0, 0..1);
        }
    }
}