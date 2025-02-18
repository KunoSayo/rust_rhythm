use crate::engine::prelude::*;
use crate::engine::render_ext::CommandEncoderExt;
use crate::engine::renderer::StaticRendererData;
use crate::engine::uniform::{bind_uniform_group, uniform_bind_buffer_layout_entry};
use bytemuck::{cast_slice, Pod, Zeroable};
use egui::Rect;
use nalgebra::{Vector2, Vector4};
use std::mem::size_of;
use std::num::NonZeroU64;
use wgpu::util::{BufferInitDescriptor, DeviceExt, RenderEncoder, StagingBelt};

#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone, Debug)]
pub struct TextureObjectVertex {
    position: Vector2<f32>,
    tex_coords: Vector2<f32>,
}

impl TextureObjectVertex {
    pub fn new(position: Vector2<f32>, tex_coords: Vector2<f32>) -> Self {
        Self {
            position,
            tex_coords,
        }
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone, Debug)]
pub struct TextureObject {
    // 48 * 2 Bytes
    vertices: [TextureObjectVertex; 4],
}

impl TextureObject {
    pub fn new_rect(
        left_up: Vector2<f32>,
        right_bottom: Vector2<f32>,
        tex: &[Vector2<f32>; 4],
    ) -> Self {
        let right_up = Vector2::new(right_bottom.x, left_up.y);
        let left_bottom = Vector2::new(left_up.x, right_bottom.y);
        let pos = [left_up, right_up, left_bottom, right_bottom];
        let tcs = &tex;
        let c = TextureObjectVertex::new;
        Self {
            vertices: [
                c(pos[0], tcs[0]),
                c(pos[1], tcs[1]),
                c(pos[2], tcs[2]),
                c(pos[3], tcs[3]),
            ],
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
    /// Group 0.
    pub sampler_bind_layout: BindGroupLayout,
    /// Group 1.
    /// Bindings 0: texture view
    pub texture_bind_layout: BindGroupLayout,
    /// Group 2.
    pub tint_bind_layout: BindGroupLayout,
    /// group 0 for nearest sampler.
    pub sampler_bind_group: BindGroup,
    pub white_tint_bg: BindGroup,
    pub normal_rp: RenderPipeline,
    pub white_tint_uniform: Buffer,
    pub stream_draw_vertex_buffer: Buffer,
}

impl TextureRenderer {
    pub fn new(gpu: &WgpuData) -> Self {
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
            }],
        });

        let tint_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[uniform_bind_buffer_layout_entry(
                0,
                ShaderStages::FRAGMENT,
                size_of::<FragUniform>() as u64,
            )],
        });

        let frag_uniform = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: cast_slice(&[FragUniform {
                tint: Vector4::new(1.0, 1.0, 1.0, 1.0),
            }]),
        });

        let rp_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&base_bind_layout, &obj_layout, &tint_layout],
            push_constant_ranges: &[],
        });
        let targets = [Some(ColorTargetState {
            format: gpu.surface_cfg.format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let shader_desc = include_wgsl!("texture_renderer.wgsl");
        let shader = device.create_shader_module(shader_desc);
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
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                strip_index_format: Some(IndexFormat::Uint16),
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

        let white_tint_bg = bind_uniform_group(device, &tint_layout, 0, &frag_uniform);
        Self {
            sampler_bind_layout: base_bind_layout,
            texture_bind_layout: obj_layout,
            tint_bind_layout: tint_layout,
            sampler_bind_group: base_bind_group,
            white_tint_bg,
            normal_rp,
            white_tint_uniform: frag_uniform,
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
    pub fn setup(&self, rp: &mut RenderPass) {
        rp.set_pipeline(&self.normal_rp);
        rp.set_bind_group(0, &self.sampler_bind_group, &[]);
    }

    /// Should call staging belt `finish()` after render.
    pub fn render<'a>(
        &'a self,
        device: &Device,
        encoder: &mut CommandEncoder,
        srd: &StaticRendererData,
        staging_belt: &mut StagingBelt,
        tex_bg: &BindGroup,
        tint_bg: &BindGroup,
        objs: &'a [TextureObject],
        target: &TextureView,
        viewport: &Rect,
    ) {
        for x in objs.chunks(256) {
            let size = x.len() * size_of::<TextureObject>();

            let mut buffer = staging_belt.write_buffer(
                encoder,
                &self.stream_draw_vertex_buffer,
                0,
                NonZeroU64::new(size as u64).unwrap(),
                device,
            );
            buffer[..size].copy_from_slice(cast_slice(&x[..]));
            drop(buffer);

            let mut rp = encoder.begin_normal(target);
            self.setup(&mut rp);
            rp.set_viewport(
                viewport.left(),
                viewport.top(),
                viewport.width(),
                viewport.height(),
                0.0,
                1.0,
            );
            rp.set_bind_group(1, tex_bg, &[]);
            rp.set_bind_group(2, tint_bg, &[]);
            rp.set_index_buffer(srd.rect_index_buffer.slice(..), IndexFormat::Uint16);
            rp.set_vertex_buffer(0, self.stream_draw_vertex_buffer.slice(..));
            rp.draw_indexed(0..(x.len() * 5) as u32, 0, 0..1);
        }
    }
}
