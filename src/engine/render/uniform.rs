use std::mem::size_of;
use std::num::NonZeroU64;
use std::slice::from_ref;
use std::sync::Arc;

use wgpu::util::{align_to, BufferInitDescriptor, DeviceExt, StagingBelt};

use crate::engine::prelude::*;
use crate::engine::render::camera::CameraUniform;

#[allow(unused)]
#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformBufferInstance {
    pub camera: CameraUniform,
    pub size: [f32; 2],
}

#[derive(Debug)]
pub struct MainUniformBuffer {
    pub data: UniformBufferInstance,
    /// Contains camera matrix and 2 more floats as screen size
    pub uniform_buffer: Arc<Buffer>,
    pub screen_uni_bind_layout: BindGroupLayout,
    pub camera_uni_bind_layout: BindGroupLayout,

    screen_offset: BufferAddress,
}

pub const CAMERA_BIND_GROUP_ENTRY: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 0,
    visibility: ShaderStages::VERTEX,
    ty: BindingType::Buffer {
        ty: BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: NonZeroU64::new(size_of::<CameraUniform>() as _),
    },
    count: None,
};

impl MainUniformBuffer {
    pub fn new(device: &Device) -> Self {
        let screen_offset = align_to(
            size_of::<CameraUniform>() as BufferAddress,
            device.limits().min_uniform_buffer_offset_alignment as BufferAddress,
        );
        let uniform_buffer = device
            .create_buffer(&BufferDescriptor {
                // Camera screen buffer
                label: Some("C.S.B."),
                size: size_of::<[f32; 2]>() as BufferAddress + screen_offset,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
            .into();
        let screen_uni_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(size_of::<[f32; 2]>() as _),
                },
                count: None,
            }],
        });

        let camera_uni_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[CAMERA_BIND_GROUP_ENTRY],
        });

        Self {
            data: UniformBufferInstance::default(),
            uniform_buffer,
            screen_uni_bind_layout,
            camera_uni_bind_layout,
            screen_offset,
        }
    }

    #[inline]
    /// Write the uniform data to buffer but not submit
    pub fn update(&self, queue: &Queue) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(from_ref(&self.data.camera)),
        );
        queue.write_buffer(
            &self.uniform_buffer,
            self.screen_offset,
            bytemuck::cast_slice(from_ref(&self.data.size)),
        );
    }

    #[inline]
    /// Write the uniform data to buffer but not submit
    pub fn update_staging(
        &self,
        device: &Device,
        ce: &mut CommandEncoder,
        staging: &mut StagingBelt,
    ) {
        let data = bytemuck::cast_slice(from_ref(&self.data.camera));
        let mut view = staging.write_buffer(
            ce,
            &self.uniform_buffer,
            0,
            BufferSize::new(data.len() as _).unwrap(),
            device,
        );
        view[..data.len()].copy_from_slice(data);
    }
}

pub fn uniform_bind_buffer_layout_entry(
    binding: u32,
    visibility: ShaderStages,
    size: u64,
) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(size),
        },
        count: None,
    }
}

pub fn uniform_bind_buffer_entry(binding: u32, buffer: &Buffer) -> BindGroupEntry {
    BindGroupEntry {
        binding,
        resource: BindingResource::Buffer(BufferBinding {
            buffer,
            offset: 0,
            size: None,
        }),
    }
}

pub fn bind_uniform_group(
    device: &Device,
    layout: &BindGroupLayout,
    binding: u32,
    buffer: &Buffer,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout,
        entries: &[uniform_bind_buffer_entry(binding, buffer)],
    })
}

pub fn create_static_uniform_buffer(device: &Device, data: &[impl bytemuck::NoUninit]) -> Buffer {
    device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(data),
        usage: BufferUsages::UNIFORM,
    })
}
