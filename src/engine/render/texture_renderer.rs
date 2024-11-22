use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use nalgebra::{Vector2, Vector4};

use crate::engine::prelude::*;
use crate::engine::uniform::uniform_bind_buffer_layout_entry;

#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone, Debug)]
pub struct RendererVertex {
    position: Vector2<f32>,
    tex_coords: Vector2<f32>,
}


#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone)]
pub struct FragUniform {
    pub tint: Vector4<f32>,
}

impl Vertex for RendererVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        const LAYOUT: VertexBufferLayout = VertexBufferLayout {
            array_stride: size_of::<RendererVertex>() as _,
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
    pub bindgroup_zero: BindGroup,
    pub normal_rp: RenderPipeline,

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
                entry_point: "vs",
                compilation_options: Default::default(),
                buffers: &[RendererVertex::desc()],
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
                entry_point: "fs",
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
            bindgroup_zero: base_bind_group,
            normal_rp,
        }
    }
}
