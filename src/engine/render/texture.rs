use image::GenericImageView;
use wgpu::{AddressMode, Device, FilterMode, Queue, Sampler, SamplerDescriptor, SurfaceConfiguration, Texture, TextureFormat, TextureView};
use wgpu::util::{DeviceExt, TextureDataOrder};

#[allow(unused)]
#[derive(Debug)]
pub struct TextureWrapper {
    pub texture: Texture,
    pub view: TextureView,
    pub info: TextureInfo,
}


#[derive(Default, Debug, Copy, Clone)]
pub struct TextureInfo {
    pub width: u32,
    pub height: u32,
}

#[allow(unused)]
impl TextureInfo {
    pub(crate) fn new(width: u32, height: u32) -> TextureInfo {
        Self { width, height }
    }
}


#[allow(unused)]
impl TextureWrapper {
    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[TextureFormat::Depth32Float],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());


        Self { texture, view, info: TextureInfo::new(size.width, size.height) }
    }

    pub fn create_depth_stencil_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Depth32FloatStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[TextureFormat::Depth32FloatStencil8],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());


        Self { texture, view, info: TextureInfo::new(size.width, size.height) }
    }

    pub fn new_with_size(device: &Device, format: TextureFormat, size: (u32, u32)) -> Self {
        let size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[format],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view, info: TextureInfo::new(size.width, size.height) }
    }

    pub fn new_multisample(device: &Device, cfg: &SurfaceConfiguration, sample_count: u32) -> Self {
        let size = wgpu::Extent3d {
            width: cfg.width,
            height: cfg.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: cfg.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[cfg.format],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view, info: TextureInfo::new(size.width, size.height) }
    }

    pub fn from_bytes(device: &Device, queue: &Queue, bytes: &[u8], label: Option<&str>, flip_y: bool) -> anyhow::Result<Self> {
        let img = image::load_from_memory(bytes)?;
        let img = if flip_y {
            img.flipv()
        } else {
            img
        };
        Self::from_image(device, queue, &img, label)
    }

    pub fn from_image(device: &Device, queue: &Queue, img: &image::DynamicImage, label: Option<&str>,
    ) -> anyhow::Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture_with_data(queue, &wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[TextureFormat::Rgba8Unorm],
        }, TextureDataOrder::default(), rgba.as_ref());

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self { texture, view, info: TextureInfo::new(size.width, size.height) })
    }

    pub fn create_linear_sampler(device: &Device) -> Sampler {
        device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            ..Default::default()
        })
    }

    pub fn create_nearest_sampler(device: &Device) -> Sampler {
        device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            lod_max_clamp: 0.0,
            ..Default::default()
        })
    }
}
