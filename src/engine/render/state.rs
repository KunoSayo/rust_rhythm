use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use anyhow::anyhow;
use egui::Rect;
use futures::executor::block_on;
use wgpu::*;
use winit::window::Window;

use crate::engine::manager::{EngineEventLoopProxy, EngineEventLoopProxyExt};
use crate::engine::render::INSTANCE;
use crate::engine::renderer::StaticRendererData;
use crate::engine::uniform::MainUniformBuffer;
use crate::engine::MainRenderViews;

#[derive(Debug)]
pub struct WgpuData {
    pub surface: Surface<'static>,
    pub surface_cfg: SurfaceConfiguration,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub views: MainRenderViews,
    pub uniforms: MainUniformBuffer,
    pub data: StaticRendererData,
    pub size_scale: [f32; 2],
}

impl WgpuData {
    #[inline]
    pub fn get_screen_size(&self) -> (u32, u32) {
        (self.surface_cfg.width, self.surface_cfg.height)
    }

    #[inline]
    pub fn get_screen_size_f32(&self) -> (f32, f32) {
        (self.surface_cfg.width as f32, self.surface_cfg.height as f32)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_cfg.width = width;
        self.surface_cfg.height = height;
        self.surface.configure(&self.device, &self.surface_cfg);
        let size = [width as f32, height as f32];
        self.size_scale = [size[0] / 1600.0, size[1] / 900.0];
        self.views = MainRenderViews::new(&self.device, &self.surface_cfg);
    }

    pub fn create_from_exists(window: &Window, gpu: &WgpuData) -> anyhow::Result<Self> {
        let window = AssertUnwindSafe(&window);
        let gpu = AssertUnwindSafe(&gpu);
        let result = std::panic::catch_unwind(|| {
            log::info!("New graphics state");
            let size = window.inner_size();
            log::info!("Got window inner size {:?}", size);

            log::info!("Got wgpu  instance {:?}", INSTANCE);
            log::info!("Window is visible, try surface.");
            let surface = unsafe {
                std::mem::transmute::<_, Surface<'static>>(INSTANCE.create_surface(window.0)?)
            };
            log::info!("Created surface {:?}", surface);

            let (device, queue) = (gpu.device.clone(), gpu.queue.clone());
            log::info!("Cloned device {:?} and queue {:?}", device, queue);

            let format = TextureFormat::Bgra8Unorm;
            log::info!("Using {:?} for swap chain format", format);

            let surface_cfg = SurfaceConfiguration {
                usage: TextureUsages::COPY_DST,
                format,
                width: size.width,
                height: size.height,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: Default::default(),
                view_formats: vec![format],
            };
            surface.configure(&device, &surface_cfg);

            let mut uniforms = MainUniformBuffer::new(&device);
            uniforms.uniform_buffer = gpu.uniforms.uniform_buffer.clone();
            let size_scale = [
                surface_cfg.width as f32 / 1600.0,
                surface_cfg.height as f32 / 900.0,
            ];
            let views = MainRenderViews::new(&device, &surface_cfg);
            let data = gpu.data.clone();
            Ok(Self {
                surface,
                surface_cfg,
                device,
                queue,
                views,

                uniforms,
                data,
                size_scale,
            })
        });
        if let Ok(r) = result {
            return r;
        }
        log::warn!("Failed to get gpu data");
        Err(anyhow!("Get gpu data failed"))
    }

    pub fn new(window: &Window, el: &impl EngineEventLoopProxy) -> anyhow::Result<Self> {
        el.run_loop_task_with_result(Box::new(|_| {
            log::info!("New graphics state");
            let size = window.inner_size();
            log::info!("Got window inner size {:?}", size);

            log::info!("Got wgpu  instance {:?}", INSTANCE);
            log::info!("Window is visible, try surface.");

            let surface = unsafe {
                INSTANCE
                    .create_surface(std::mem::transmute::<_, &'static Window>(window))
                    .map_err(|e| anyhow!("Create surface failed for {e:?}"))
            }?;

            log::info!("Created surface {:?}", surface);
            let adapter = block_on(INSTANCE.request_adapter(&RequestAdapterOptions {
                power_preference:
                    PowerPreference::from_env().unwrap_or(PowerPreference::HighPerformance),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .ok_or(anyhow!("Cannot get adapter"))?;
            log::info!("Got adapter {:?}", adapter);
            let (device, queue) = block_on(adapter.request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::default(),
                    required_limits: Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            ))?;

            let (device, queue) = (Arc::new(device), Arc::new(queue));
            log::info!("Requested device {:?} and queue {:?}", device, queue);

            let format = TextureFormat::Bgra8Unorm;
            log::info!("Using {:?} for swap chain format", format);

            let surface_cfg = SurfaceConfiguration {
                usage: TextureUsages::COPY_DST,
                format,
                width: size.width,
                height: size.height,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 0,
                alpha_mode: Default::default(),
                view_formats: vec![format],
            };
            surface.configure(&device, &surface_cfg);

            let uniforms = MainUniformBuffer::new(&device);
            let size_scale = [
                surface_cfg.width as f32 / 1600.0,
                surface_cfg.height as f32 / 900.0,
            ];
            let views = MainRenderViews::new(&device, &surface_cfg);
            let data = StaticRendererData::new(&device);
            Ok(Self {
                surface,
                surface_cfg,
                device,
                queue,
                views,
                uniforms,
                data,
                size_scale,
            })
        }))
    }

    pub fn new_async(window: &Window, el: &impl EngineEventLoopProxy) -> anyhow::Result<Self> {
        log::info!("New graphics state");
        let size = window.inner_size();
        log::info!("Got window inner size {:?}", size);

        log::info!("Got wgpu  instance {:?}", INSTANCE);
        log::info!("Window is visible, try surface.");

        let surface = el.run_loop_task_with_result(Box::new(|_| unsafe {
            INSTANCE
                .create_surface(std::mem::transmute::<_, &'static Window>(window))
                .map_err(|e| anyhow!("Create surface failed for {e:?}"))
        }))?;

        log::info!("Created surface {:?}", surface);
        let adapter = block_on(INSTANCE.request_adapter(&RequestAdapterOptions {
            power_preference:
                PowerPreference::from_env().unwrap_or(PowerPreference::HighPerformance),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .ok_or(anyhow!("Cannot get adapter"))?;
        log::info!("Got adapter {:?}", adapter);
        let (device, queue) = block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::default(),
                required_limits: Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))?;

        let (device, queue) = (Arc::new(device), Arc::new(queue));
        log::info!("Requested device {:?} and queue {:?}", device, queue);

        let format = TextureFormat::Bgra8Unorm;
        log::info!("Using {:?} for swap chain format", format);

        let surface_cfg = SurfaceConfiguration {
            usage: TextureUsages::COPY_DST,
            format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 0,
            alpha_mode: Default::default(),
            view_formats: vec![format],
        };
        surface.configure(&device, &surface_cfg);

        let uniforms = MainUniformBuffer::new(&device);
        let size_scale = [
            surface_cfg.width as f32 / 1600.0,
            surface_cfg.height as f32 / 900.0,
        ];
        let views = MainRenderViews::new(&device, &surface_cfg);
        let data = StaticRendererData::new(&device);
        Ok(Self {
            surface,
            surface_cfg,
            device,
            queue,
            views,
            uniforms,
            data,
            size_scale,
        })
    }
}
