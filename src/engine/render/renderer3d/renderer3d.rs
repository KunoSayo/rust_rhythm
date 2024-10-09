//! Render 3d renderer3d with texture without lights.
//!
//! Vertex use world pos.
//!
//! Use global camera uniform

use std::array::from_ref;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use nalgebra::{vector, Vector2, Vector3};
use wgpu::util::{BufferInitDescriptor, DeviceExt, RenderEncoder};

use crate::engine::prelude::*;
use crate::engine::uniform::{uniform_bind_buffer_layout_entry, CAMERA_BIND_GROUP_ENTRY};

#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone, Debug)]
pub struct PlaneVertex {
    pub pos: Vector3<f32>,
    pub tex_coord: Vector2<f32>,
    pub normal: Vector3<f32>,
}


#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone)]
pub struct LightUniform {
    pub light: Vector3<f32>,
    pub width: f32,
    pub dir: Vector3<f32>,
    pub height: f32,
}

#[repr(C)]
#[derive(Pod, Zeroable, Default, Copy, Clone, Debug)]
pub struct PlaneObject {
    pub vertex: [PlaneVertex; 4],
}

impl PlaneObject {
    pub fn new(center: &Vector3<f32>, r: f32, tex_center: &Vector2<f32>, tex_delta: f32, up: &Vector3<f32>, right: &Vector3<f32>) -> Self {
        let forward = up.cross(right);
        let vertex = (0..4).map(|i| {
            // left right
            // left right
            let axis_forward = if i < 2 {
                forward * r
            } else {
                -forward * r
            };
            let axis_left = if i & 1 == 0 {
                right * r
            } else {
                -right * r
            };

            let tex_coord = vector![if i < 2 {
                tex_center.x + tex_delta
            } else {
                tex_center.x - tex_delta
            }, if i & 1 == 0 {
                tex_center.y + tex_delta
            } else {
                tex_center.y - tex_delta
            }];
            PlaneVertex {
                pos: axis_left + axis_forward + center,
                tex_coord,
                normal: *up,
            }
        }).collect::<Vec<_>>().try_into().unwrap();
        Self {
            vertex,
        }
    }
}

impl Vertex for PlaneVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<[f32; 8]>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }, VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 12,
                shader_location: 1,
            }, VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 20,
                shader_location: 2,
            }],
        }
    }
}


// group 0 for base layout: camera sampler light
// group 1 for planes using the same texture
pub struct PlaneRenderer {
    /// Group0.
    pub base_bind_layout: BindGroupLayout,
    /// Group1.
    /// Bindings 0: texture view
    pub obj_layout: BindGroupLayout,
    pub light_uniform: Buffer,
    pub bindgroup_zero: BindGroup,
    pub normal_rp: RenderPipeline,
    pub no_cull_rp: RenderPipeline,
    pub screen_tex_no_cull_rp: RenderPipeline,
    pub depth_only_rp: RenderPipeline,
}

#[derive(Debug)]
pub struct Planes {
    pub objs: Vec<PlaneObject>,
    pub texture_bind: Option<BindGroup>,
}

#[derive(Debug)]
pub struct StaticPlanes {
    pub count: u32,
    pub buffer: Buffer,
    pub texture_bind: Option<BindGroup>,
}


impl Planes {
    pub fn to_static(self, device: &Device) -> StaticPlanes {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&self.objs[..]),
            usage: BufferUsages::VERTEX,
        });
        StaticPlanes {
            count: self.objs.len() as u32,
            buffer,
            texture_bind: self.texture_bind,
        }
    }
}

impl PlaneRenderer {
    pub fn new(gpu: &WgpuData, shader: &ShaderModule) -> Self {
        let device = &gpu.device;
        let base_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("plane uniform layout"),
            entries: &[CAMERA_BIND_GROUP_ENTRY,
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                }, uniform_bind_buffer_layout_entry(2, ShaderStages::FRAGMENT, size_of::<LightUniform>() as _)],
        });
        let obj_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("plane obj layout"),
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

        let sampler = TextureWrapper::create_nearest_sampler(&device);


        let light_uniform = device.create_buffer(&BufferDescriptor {
            label: None,
            size: size_of::<LightUniform>() as _,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });


        let bindgroup_zero = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &base_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: gpu.uniforms.uniform_buffer.as_entire_binding(),
            }, BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&sampler),
            }, BindGroupEntry {
                binding: 2,
                resource: light_uniform.as_entire_binding(),
            }],
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
        let mut rpd = RenderPipelineDescriptor {
            label: None,
            layout: Some(&rp_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "plane_vs",
                compilation_options: Default::default(),
                buffers: &[PlaneVertex::desc()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "plane_fs",
                compilation_options: Default::default(),
                targets: &targets,
            }),
            multiview: None,
            cache: None,
        };
        let normal_rp = device.create_render_pipeline(&rpd);
        rpd.primitive.cull_mode = None;
        let no_cull_rp = device.create_render_pipeline(&rpd);
        rpd.primitive.cull_mode = Some(Face::Back);


        rpd.primitive.cull_mode = None;
        rpd.vertex.entry_point = "plane_vs_full_tex";
        rpd.fragment.as_mut().unwrap().entry_point = "plane_pos_tex_fs";
        let screen_tex_no_cull_rp = device.create_render_pipeline(&rpd);

        rpd.fragment = None;
        let rp_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&base_bind_layout],
            push_constant_ranges: &[],
        });
        rpd.layout = Some(&rp_layout);

        rpd.vertex.entry_point = "plane_vs";
        let depth_only_rp = device.create_render_pipeline(&rpd);
        Self {
            base_bind_layout,
            obj_layout,
            light_uniform,
            bindgroup_zero,
            normal_rp,
            no_cull_rp,
            screen_tex_no_cull_rp,
            depth_only_rp,
        }
    }

    pub fn create_plane(&self, device: &Device, tv: Option<&TextureView>) -> Planes {
        let texture_bind = tv.map(|tv| device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.obj_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(tv),
            }],
        }));
        Planes {
            objs: vec![],
            texture_bind,
        }
    }

    pub fn update_light(&mut self, queue: &Queue, light: &LightUniform) {
        queue.write_buffer(&self.light_uniform, 0, bytemuck::cast_slice(from_ref(light)));
    }
}

#[allow(unused)]
pub struct General3DRenderer {
    pub plane_renderer: PlaneRenderer,
}

#[allow(unused)]
impl General3DRenderer {
    pub fn new(gpu: &WgpuData) -> Self {
        let device = &gpu.device;
        // Setup the shader
        // We use specific shaders for each pass to define visual effect
        // and also to have the right shader for the uniforms we pass
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("General 3d Shader"),
            source: ShaderSource::Wgsl(include_str!("3d.wgsl").into()),
        });
        let plane_renderer = PlaneRenderer::new(gpu, &shader_module);
        Self {
            plane_renderer,
        }
    }
}

#[allow(unused)]
impl PlaneRenderer {
    #[inline]
    pub fn bind<'a, T: RenderEncoder<'a>>(&'a self, encoder: &mut T) {
        encoder.set_bind_group(0, &self.bindgroup_zero, &[]);
    }


    pub fn render_static<'a, T: RenderEncoder<'a>>(&'a self, encoder: &mut T, _: &WgpuData, objs: &'a [StaticPlanes]) {
        for obj in objs {
            if let Some(bg) = &obj.texture_bind {
                encoder.set_bind_group(1, bg, &[]);
            }
            encoder.set_vertex_buffer(0, obj.buffer.slice(..));
            for i in 0..obj.count {
                let start = i * 4;
                let end = (i + 1) * 4;
                encoder.draw(start..end, 0..1);
            }
        }
    }
}
