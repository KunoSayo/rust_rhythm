use egui::epaint::ahash::{HashMap, HashMapExt};
use egui::Context;
use egui_wgpu::ScreenDescriptor;
use futures::executor::{block_on, LocalSpawner, ThreadPool};
use futures::future::RemoteHandle;
use futures::task::SpawnExt;
use futures::FutureExt;
use log::info;
use specs::World;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::default::Default;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender};
use std::task::Poll;
use std::thread::JoinHandle;
use std::time::Instant;
use wgpu::{
    Color, CommandEncoderDescriptor, Extent3d, ImageCopyTexture, LoadOp, Maintain, Operations,
    Origin3d, RenderPassColorAttachment, RenderPassDescriptor, StoreOp, TexelCopyTextureInfo,
    TextureAspect,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalSize, Size};
use winit::error::OsError;
use winit::event::{ElementState, Event, MouseButton, StartCause, TouchPhase, WindowEvent};
use winit::event_loop::{
    ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop, EventLoopBuilder, EventLoopProxy,
};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::engine::app::AppInstance;
use crate::engine::global::IO_POOL;
use crate::engine::prelude::MaintainBase;
use crate::engine::{
    GameState, GlobalData, LoopState, MainRendererData, MouseState, Pointer, StateEvent, Trans,
    WgpuData,
};

#[derive(Default)]
struct LoopInfo {
    pressed_keys: HashSet<PhysicalKey>,
    released_keys: HashSet<PhysicalKey>,
    mouse_input: MouseState,
    loop_state: LoopState,
    got_event: bool,
}

impl LoopInfo {
    pub(crate) fn updated(&mut self) {
        self.got_event = false;
    }
}

pub struct WindowInstance {
    pub id: WindowId,
    pub app: AppInstance,
    pub states: Vec<Box<dyn GameState>>,
    running: bool,
    loop_info: LoopInfo,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum WinitEventLoopMessage {
    WakeUp(WindowId),
    CheckAsyncMsg,
}

pub type EventLoopTargetType = ActiveEventLoop;
pub type EventLoopProxyType = EventLoopProxy<WinitEventLoopMessage>;

impl WindowInstance {
    pub fn is_running(&self) -> bool {
        self.running
    }
    #[allow(unused)]
    pub fn stop_run(&mut self) {
        self.running = false;
    }
}

#[allow(unused)]
impl WindowInstance {
    pub fn new_with_gpu(
        title: &str,
        setup: impl FnOnce(WindowAttributes) -> WindowAttributes,
        el: impl EngineEventLoopProxy,
        gpu: &WgpuData,
    ) -> anyhow::Result<Self> {
        let window = el.create_window(setup(WindowAttributes::default()).with_title(title))?;
        let id = window.id();
        let app = AppInstance::create_from_gpu(window, gpu)?;
        Ok(Self {
            id,
            app,
            states: vec![],
            running: true,
            loop_info: Default::default(),
        })
    }

    pub fn new(
        title: &str,
        setup: impl FnOnce(WindowAttributes) -> WindowAttributes,
        el: impl EngineEventLoopProxy,
    ) -> anyhow::Result<Self> {
        let window = el.create_window(setup(WindowAttributes::default().with_title(title)))?;
        let id = window.id();
        let app = AppInstance::new(window, &el)?;
        Ok(Self {
            id,
            app,
            states: vec![],
            running: true,
            loop_info: Default::default(),
        })
    }

    pub fn new_from_window(window: Window, el: &impl EngineEventLoopProxy) -> anyhow::Result<Self> {
        Ok(Self {
            id: window.id(),
            app: AppInstance::new(window, el)?,
            states: vec![],
            running: true,
            loop_info: Default::default(),
        })
    }
}
/// put app and el here
macro_rules! get_state {
    ($app: expr_2021, $el: expr_2021) => {{
        crate::engine::state::StateData {
            app: &mut $app,
            wd: $el,
            dt: 0.0,
        }
    }};
}

impl WindowInstance {
    fn loop_once(&mut self, wd: &mut GlobalData) {
        profiling::scope!("Loop logic once");
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.app.last_update_time).as_secs_f32();

        self.loop_info.loop_state.reset(0.0);
        self.app.inputs.mouse_state = self.loop_info.mouse_input;
        self.loop_info.mouse_input.last_left_click = self.loop_info.mouse_input.left_click;
        self.app.inputs.swap_frame();

        {
            let mut state_data = get_state!(self.app, wd);
            state_data.dt = dt;
            for x in &mut self.states {
                self.loop_info.loop_state |= x.shadow_update(&mut state_data);
            }
            if let Some(last) = self.states.last_mut() {
                let ((tran, l), wd) = { (last.update(&mut state_data), state_data.wd) };
                self.process_tran(tran, wd);
                self.loop_info.loop_state |= l;
            }
        }
        self.app.last_update_time = now;
    }

    fn process_tran(&mut self, tran: Trans, el: &mut GlobalData) {
        let last = self.states.last_mut().unwrap();
        let mut state_data = get_state!(self.app, el);
        match tran {
            Trans::Push(mut x) => {
                self.loop_info.loop_state |= x.start(&mut state_data);
                self.states.push(x);
            }
            Trans::Pop => {
                last.stop(&mut state_data);
                self.states.pop().unwrap();
                if let Some(state) = self.states.last_mut() {
                    state.on_event(&mut state_data, StateEvent::Resume);
                }
            }
            Trans::Switch(x) => {
                last.stop(&mut state_data);
                *last = x;
                self.loop_info.loop_state |= last.start(&mut state_data);
            }
            Trans::Exit => {
                while let Some(mut last) = self.states.pop() {
                    last.stop(&mut state_data);
                }
                self.running = false;
            }
            Trans::Vec(ts) => {
                for t in ts {
                    self.process_tran(t, el);
                }
            }
            Trans::None => {}
        }
    }

    fn render_once(&mut self, el: &mut GlobalData) {
        match (&self.app.gpu,) { (Some(gpu),) => {
            profiling::scope!("Render once");
            let render_now = Instant::now();
            let render_dur = render_now.duration_since(self.app.last_render_time);
            let dt = render_dur.as_secs_f32();
            self.loop_info.loop_state.reset(dt);
            {
                let mut encoder = gpu
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Clear Encoder"),
                    });
                let _ = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &gpu.views.get_screen().view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                gpu.queue.submit(Some(encoder.finish()));
            }

            let full_output = self.app.egui_ctx.clone().run(
                self.app.egui.take_egui_input(&self.app.window),
                |egui_ctx| {
                    let mut state_data = get_state!(self.app, el);
                    state_data.dt = dt;

                    for game_state in &mut self.states {
                        game_state.shadow_render(&mut state_data, egui_ctx);
                    }
                    if let Some(g) = self.states.last_mut() {
                        let tran = g.render(&mut state_data, egui_ctx);
                        self.process_tran(tran, el);
                    }
                },
            );

            let gpu = self.app.gpu.as_ref().unwrap();
            let render = self.app.render.as_mut().unwrap();
            // render ui output to main screen
            {
                let device = gpu.device.as_ref();
                let queue = gpu.queue.as_ref();
                let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("encoder for egui"),
                });

                let screen_descriptor = ScreenDescriptor {
                    size_in_pixels: [gpu.surface_cfg.width, gpu.surface_cfg.height],
                    pixels_per_point: self.app.window.scale_factor() as f32,
                };
                // Upload all resources for the GPU.

                let egui_renderer = &mut render.egui_rpass;
                let paint_jobs = self
                    .app
                    .egui
                    .egui_ctx()
                    .tessellate(full_output.shapes, 1.0f32);
                for (id, delta) in &full_output.textures_delta.set {
                    egui_renderer.update_texture(device, queue, *id, &delta);
                }
                let buffer = egui_renderer.update_buffers(
                    &device,
                    &queue,
                    &mut encoder,
                    &paint_jobs,
                    &screen_descriptor,
                );
                queue.submit(buffer);
                {
                    let mut rp = encoder
                        .begin_render_pass(&RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(RenderPassColorAttachment {
                                view: &gpu.views.get_screen().view,
                                resolve_target: None,
                                ops: Operations {
                                    load: LoadOp::Load,
                                    store: StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        })
                        .forget_lifetime();
                    egui_renderer.render(&mut rp, &paint_jobs, &screen_descriptor);
                }
                // Submit the commands.
                queue.submit(std::iter::once(encoder.finish()));
                full_output
                    .textures_delta
                    .free
                    .iter()
                    .for_each(|id| egui_renderer.free_texture(id));
            }

            {
                let mut sd = get_state!(self.app, el);
                sd.dt = dt;
                self.states
                    .iter_mut()
                    .for_each(|s| s.on_event(&mut sd, StateEvent::PostUiRender));
            }
            let gpu = self.app.gpu.as_ref().unwrap();

            // We do get here

            let swap_chain_frame = match gpu.surface.get_current_texture() { Ok(s) => {
                s
            } _ => {
                // it is normal.
                log::trace!("Lose swap chain frame!");
                return;
            }};

            {
                let mut encoder = gpu
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Copy buffer to screen commands"),
                    });
                let size = gpu.get_screen_size();

                let surface_output = &swap_chain_frame;
                encoder.copy_texture_to_texture(
                    TexelCopyTextureInfo {
                        texture: &gpu.views.get_screen().texture,
                        mip_level: 0,
                        origin: Origin3d::default(),
                        aspect: TextureAspect::All,
                    },
                    TexelCopyTextureInfo {
                        texture: &surface_output.texture,
                        mip_level: 0,
                        origin: Default::default(),
                        aspect: TextureAspect::All,
                    },
                    Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                );
                gpu.queue.submit(Some(encoder.finish()));
            }

            // if self.window.inputs.is_pressed(&[VirtualKeyCode::F11]) {
            //     self.window.save_screen_shots();
            // }
            //
            // self.window.pools.render_pool.try_run_one();

            self.app.last_render_time = render_now;
            swap_chain_frame.present();

            self.app
                .egui
                .handle_platform_output(&self.app.window, full_output.platform_output);
            self.app.render.as_mut().unwrap().staging_belt.recall();
        } _ => {
            // no gpu but we need render it...
            // well...
            // no idea.
        }}
    }

    fn start(&mut self, mut start: Box<dyn GameState>, wd: &mut GlobalData) {
        start.start(&mut get_state!(self.app, wd));
        info!("Started the start state.");
        self.states.push(start);
    }

    fn on_window_event(&mut self, we: &WindowEvent, wd: &mut GlobalData) {
        self.loop_info.got_event = true;
        // let _ = self.app.egui.on_window_event(&self.app.window, we);

        let sd = &mut get_state!(self.app, wd);
        for x in &mut self.states {
            x.on_event(sd, StateEvent::Window(we));
        }
        match we {
            WindowEvent::Touch(touch) => {
                self.loop_info.mouse_input.pos =
                    (touch.location.x as f32, touch.location.y as f32).into();
                match touch.phase {
                    TouchPhase::Started => {
                        self.loop_info.mouse_input.left_click = true;
                        self.loop_info.mouse_input.last_left_click = false;
                    }
                    TouchPhase::Moved => {
                        self.loop_info.mouse_input.left_click = true;
                        self.loop_info.mouse_input.last_left_click = true;
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        self.loop_info.mouse_input.left_click = false;
                        self.loop_info.mouse_input.last_left_click = true;
                    }
                }
                self.app
                    .inputs
                    .points
                    .insert(touch.id, Pointer::from(*touch));
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.loop_info.mouse_input.pos = (position.x as f32, position.y as f32).into();
            }
            WindowEvent::CursorLeft { .. } => {
                self.loop_info.mouse_input.pos = (-9961.0, -9961.0).into();
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                if *button == MouseButton::Left {
                    match *state {
                        ElementState::Pressed => {
                            self.loop_info.mouse_input.left_click = true;
                            self.loop_info.mouse_input.last_left_click = false;
                        }
                        ElementState::Released => {
                            self.loop_info.mouse_input.left_click = false;
                            self.loop_info.mouse_input.last_left_click = true;
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if !is_synthetic {
                    let key = event.physical_key;
                    match event.state {
                        ElementState::Pressed => {
                            self.loop_info.pressed_keys.insert(key);
                        }
                        ElementState::Released => {
                            self.loop_info.released_keys.insert(key);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct WindowManager {
    start: Option<Box<dyn GameState>>,
    root: Option<WindowId>,
    proxy: EventLoopProxyType,
    world: World,
    windows: HashMap<WindowId, RefCell<Box<WindowInstance>>>,

    all_events: usize,
    draw_events: usize,
}

impl WindowManager {
    pub(crate) fn new(elp: EventLoopProxyType) -> anyhow::Result<Self> {
        Ok(Self {
            start: None,
            root: None,
            proxy: elp,
            world: Default::default(),
            windows: Default::default(),
            all_events: 0,
            draw_events: 0,
        })
    }

    pub(crate) fn new_with_start(
        elp: EventLoopProxyType,
        start: impl GameState,
    ) -> anyhow::Result<Self> {
        let mut this = Self {
            start: None,
            root: None,
            proxy: elp,
            world: Default::default(),
            windows: Default::default(),
            all_events: 0,
            draw_events: 0,
        };
        this.start = Some(Box::new(start));
        Ok(this)
    }

    pub(crate) fn run_loop(
        mut self,
        event_loop: EventLoop<WinitEventLoopMessage>,
        start: impl GameState,
    ) {
        self.start = Some(Box::new(start));
        event_loop.listen_device_events(DeviceEvents::Never);
        let result = event_loop.run_app(&mut self);
        if let Err(e) = result {
            log::error!("Failed to run event loop for {:?}", e);
        }
    }
}

pub trait EngineEventLoopProxy {
    fn control_flow(&self) -> ControlFlow;

    fn exiting(&self) -> bool;

    fn exit(&self);

    fn create_window(&self, attr: WindowAttributes) -> Result<Window, OsError>;

    fn set_control_flow(&self, cf: ControlFlow);

    fn run_loop_task(&self, f: Box<dyn FnOnce(&ActiveEventLoop) + Send>);

    fn request_draw(&self, window: &Window);
}

impl EngineEventLoopProxy for &ActiveEventLoop {
    fn control_flow(&self) -> ControlFlow {
        ActiveEventLoop::control_flow(self)
    }

    fn exiting(&self) -> bool {
        ActiveEventLoop::exiting(self)
    }

    fn exit(&self) {
        ActiveEventLoop::exit(self)
    }

    fn create_window(&self, attr: WindowAttributes) -> Result<Window, OsError> {
        ActiveEventLoop::create_window(self, attr)
    }

    fn set_control_flow(&self, cf: ControlFlow) {
        ActiveEventLoop::set_control_flow(self, cf);
    }

    fn run_loop_task(&self, f: Box<dyn FnOnce(&ActiveEventLoop) + Send>) {
        f(self);
    }

    fn request_draw(&self, window: &Window) {
        window.request_redraw();
    }
}

impl WindowManager {
    fn on_new_events(&mut self, el: impl EngineEventLoopProxy, cause: StartCause) {
        profiling::finish_frame!();
        self.all_events = 0;
        if !matches!(cause, StartCause::WaitCancelled { .. }) {
            self.all_events = 1;
        }
        self.draw_events = 0;
    }

    fn on_resumed(&mut self, event_loop: impl EngineEventLoopProxy) {
        let mut created_windows = Vec::new();

        if self.root.is_none() {
            // root init.

            let mut attr = WindowAttributes::default()
                .with_title("Rust Rhythm")
                .with_inner_size(PhysicalSize::new(1600, 900));

            let window = event_loop
                .create_window(attr)
                .expect("Create window failed");
            let rid = window.id();
            self.root = Some(rid);
            self.windows.insert(
                window.id(),
                RefCell::new(Box::new(
                    WindowInstance::new_from_window(window, &event_loop).unwrap(),
                )),
            );

            let mut wd = GlobalData {
                el: &event_loop,
                elp: &self.proxy,
                windows: &self.windows,
                new_windows: &mut created_windows,
                world: &mut self.world,
            };
            let root_window_ins = self.windows.get(&rid).unwrap();
            root_window_ins
                .borrow_mut()
                .start(self.start.take().unwrap(), &mut wd);
            for x in created_windows {
                let id = x.app.window.id();
                self.windows.insert(id, RefCell::new(Box::new(x)));
            }

            created_windows = Vec::new();
        }

        for (_, this) in &self.windows {
            let mut this = this.borrow_mut();
            if this.app.gpu.is_none() {
                info!("gpu not found, try to init");
                this.app.gpu = match WgpuData::new(&this.app.window, &event_loop) {
                    Ok(gpu) => Some(gpu),
                    Err(err) => {
                        log::error!("Failed to create wgpu data when on resumed for {:?}", err);
                        None
                    }
                };
                if let Some(gpu) = &this.app.gpu {
                    this.app.render = Some(MainRendererData::new(gpu, &this.app.res));
                    let mut gd = GlobalData {
                        el: &event_loop,
                        elp: &self.proxy,
                        windows: &self.windows,
                        new_windows: &mut created_windows,
                        world: &mut self.world,
                    };
                    let WindowInstance {
                        app,
                        states,
                        ..
                    } = this.deref_mut().deref_mut();
                    let sd = &mut get_state!(*app, &mut gd);
                    states
                        .iter_mut()
                        .for_each(|x| x.on_event(sd, StateEvent::ReloadGPU));
                }

                // this.app.egui_ctx = Context::default();
                let size = this.app.window.inner_size();
                // this.app.egui_ctx.set_pixels_per_point(this.app.window.scale_factor() as f32);
                let WindowInstance { app, .. } = this.deref_mut().deref_mut();
                let _ = app
                    .egui
                    .on_window_event(&app.window, &WindowEvent::Resized(size));
            }
        }
        for x in created_windows {
            self.windows.insert(x.id, RefCell::new(Box::new(x)));
        }
    }

    fn on_window_event(
        &mut self,
        event_loop: impl EngineEventLoopProxy,
        window_id: WindowId,
        event: WindowEvent,
        time: Instant,
    ) {
        log::trace!(target: "winit_event", "{:?}", event);
        self.all_events += 1;
        let control_flow = event_loop.control_flow();

        if event_loop.exiting() {
            return;
        }

        let mut created_windows = Vec::new();

        {
            if let Some(window) = self.windows.get(&window_id) {
                let mut wd = GlobalData {
                    el: &event_loop,
                    elp: &self.proxy,
                    windows: &self.windows,
                    new_windows: &mut created_windows,
                    world: &mut self.world,
                };

                let mut wm = window.borrow_mut();
                wm.on_window_event(&event, &mut wd);

                let AppInstance {
                    ref window,
                    ref mut egui,
                    ..
                } = wm.app;

                if !matches!(event, WindowEvent::RedrawRequested) {
                    let _ = egui.on_window_event(window, &event);
                }
            }
        }
        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&window_id);
                if let Some(rid) = self.root {
                    if window_id == rid {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Destroyed => {
                self.windows.remove(&window_id);
                if let Some(rid) = self.root {
                    if window_id == rid {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(this) = self.windows.get_mut(&window_id) {
                    if !this.get_mut().is_running() {
                        self.windows.remove(&window_id);
                    } else if size.width > 1 && size.height > 1 {
                        let this = this.get_mut();
                        if let Some(gpu) = &mut this.app.gpu {
                            info!(
                                "Window resized ({}x{}), telling gpu data",
                                size.width, size.height
                            );
                            gpu.resize(size.width, size.height);
                            match &mut this.app.render {
                                Some(_) => {}
                                _ => {
                                    this.app.render =
                                        Some(MainRendererData::new(gpu, &this.app.res));
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // re create wgpu if not found
                if let Some(this) = self.windows.get(&window_id) {
                    let mut this = this.borrow_mut();
                    if this.app.gpu.is_none() {
                        info!("gpu not found when redraw requested, try to init");
                        this.app.gpu = match WgpuData::new(&this.app.window, &event_loop) {
                            Ok(gpu) => Some(gpu),
                            Err(err) => {
                                log::error!(
                                    "Failed to create wgpu data when on request redraw for {:?}",
                                    err
                                );
                                None
                            }
                        };
                        if let Some(gpu) = &this.app.gpu {
                            this.app.render = Some(MainRendererData::new(gpu, &this.app.res));
                            let mut gd = GlobalData {
                                el: &event_loop,
                                elp: &self.proxy,
                                windows: &self.windows,
                                new_windows: &mut created_windows,
                                world: &mut self.world,
                            };
                            let WindowInstance {
                                app,
                                states,
                                ..
                            } = this.deref_mut().deref_mut();
                            let sd = &mut get_state!(*app, &mut gd);
                            states
                                .iter_mut()
                                .for_each(|x| x.on_event(sd, StateEvent::ReloadGPU));
                        }

                        // this.app.egui_ctx = Context::default();
                        let size = this.app.window.inner_size();
                        // this.app.egui_ctx.set_pixels_per_point(this.app.window.scale_factor() as f32);
                        let WindowInstance { app, .. } = this.deref_mut().deref_mut();
                        let _ = app
                            .egui
                            .on_window_event(&app.window, &WindowEvent::Resized(size));
                    }
                }
                self.draw_events += 1;
                let mut not_running = vec![];

                match self.windows.get(&window_id) { Some(this) => {
                    let mut this = this.borrow_mut();

                    'update: {
                        let mut this = &mut this;
                        let this = this.deref_mut();
                        if !this.loop_info.got_event
                            && this.loop_info.loop_state.control_flow == ControlFlow::Wait
                        {
                            break 'update;
                        }
                        if this.states.is_empty() {
                            this.running = false;
                        }
                        if this.running {
                            let mut wd = GlobalData {
                                el: &event_loop,
                                elp: &self.proxy,
                                windows: &self.windows,
                                new_windows: &mut created_windows,
                                world: &mut self.world,
                            };
                            this.render_once(&mut wd);
                            if this.loop_info.loop_state.render > 0.0
                                || this.app.egui_ctx.has_requested_repaint()
                            {
                                this.app.window.request_redraw();
                            }
                        } else {
                            not_running.push(window_id);
                            if let Some(rid) = self.root {
                                if window_id == rid {
                                    event_loop.exit();
                                }
                            }
                        }
                    }
                } _ => {
                    if self.root.map(|id| id == window_id).unwrap_or(false) {
                        event_loop.exit()
                    }
                }}

                for id in not_running {
                    self.windows.remove(&id);
                }
            }

            _ => {}
        }

        for x in created_windows {
            self.windows.insert(x.id, RefCell::new(Box::new(x)));
        }
    }

    fn on_user_event(
        &mut self,
        event_loop: impl EngineEventLoopProxy,
        user_event: WinitEventLoopMessage,
    ) {
        match user_event {
            WinitEventLoopMessage::WakeUp(id) => {
                if let Some(this) = self.windows.get_mut(&id) {
                    event_loop.set_control_flow(ControlFlow::Poll);
                    this.get_mut().loop_info.got_event = true;
                }
            }
            WinitEventLoopMessage::CheckAsyncMsg => {
                // Nothing to check.
            }
        }
    }

    fn on_about_to_wait(&mut self, event_loop: impl EngineEventLoopProxy) {
        if event_loop.exiting() {
            return;
        }
        if self.all_events == self.draw_events {
            // not update.
            log::trace!(target:"winit_event", "Skip update due to only redraw event.");
            return;
        } else {
            log::trace!(target:"winit_event", "Update event");
        }
        let mut created_windows = Vec::new();

        let mut not_running = vec![];
        let mut f_ls = LoopState::WAIT_ALL;

        for (window_id, this) in &self.windows {
            let window_id = *window_id;
            let mut this = this.borrow_mut();
            // update logical

            'update: {
                let mut this = &mut this;
                let this = this.deref_mut();
                if !this.loop_info.got_event
                    && this.loop_info.loop_state.control_flow == ControlFlow::Wait
                {
                    break 'update;
                }
                if !this.loop_info.pressed_keys.is_empty()
                    || !this.loop_info.released_keys.is_empty()
                {
                    log::trace!(target: "InputTrace", "process window {:?} pressed_key {:?} and released {:?}", window_id, this.loop_info.pressed_keys, this.loop_info.released_keys);
                    this.app
                        .inputs
                        .process(&this.loop_info.pressed_keys, &this.loop_info.released_keys);
                    this.loop_info.pressed_keys.clear();
                    this.loop_info.released_keys.clear();
                }
                if this.states.is_empty() {
                    this.running = false;
                }
                if this.running {
                    let mut wd = GlobalData {
                        el: &event_loop,
                        elp: &self.proxy,
                        windows: &self.windows,
                        new_windows: &mut created_windows,
                        world: &mut self.world,
                    };
                    this.loop_once(&mut wd);
                    let ls = this.loop_info.loop_state;
                    if ls.render > 0.0 {
                        event_loop.request_draw(&this.app.window);
                    }
                    this.loop_info.loop_state = ls;
                    f_ls |= ls;
                } else {
                    not_running.push(window_id);
                    if let Some(rid) = self.root {
                        if window_id == rid {
                            event_loop.exit();
                        }
                    }
                }
                this.loop_info.updated();
            }
        }
        event_loop.set_control_flow(f_ls.control_flow);

        for id in not_running {
            self.windows.remove(&id);
        }
        for x in created_windows {
            self.windows.insert(x.id, RefCell::new(Box::new(x)));
        }
    }
}

impl ApplicationHandler<WinitEventLoopMessage> for WindowManager {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.on_new_events(event_loop, cause);
    }

    fn resumed(&mut self, mut event_loop: &ActiveEventLoop) {
        self.on_resumed(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, user_event: WinitEventLoopMessage) {
        self.on_user_event(event_loop, user_event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.on_window_event(event_loop, window_id, event, Instant::now())
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.on_about_to_wait(event_loop);
    }
}

#[derive(Debug)]
enum LoopMessage {
    NewLoop(StartCause),
    Resumed,
    WindowEvent(WindowId, WindowEvent, Instant),
    UserEvent(WinitEventLoopMessage),
    AboutToWait,
}

enum ToLoopMessage {
    RunTask(Box<dyn FnOnce(&ActiveEventLoop) + Send + 'static>),
}

struct AsyncWindowManagerInner {
    window_manager: RefCell<WindowManager>,
    elp: EventLoopProxyType,
    recv: Receiver<LoopMessage>,
    sender: Sender<ToLoopMessage>,
    cf: Cell<ControlFlow>,
    exiting: Cell<bool>,
    request_draws: RefCell<HashSet<WindowId>>,
}

impl AsyncWindowManagerInner {
    /// Should new in app thread.
    fn new(
        elp: EventLoopProxyType,
        rx: Receiver<LoopMessage>,
        tx: Sender<ToLoopMessage>,
        start: impl GameState,
    ) -> anyhow::Result<Self> {
        let this = Self {
            window_manager: RefCell::new(WindowManager::new_with_start(elp.clone(), start)?),
            elp,
            recv: rx,
            sender: tx,
            cf: Cell::new(Default::default()),
            exiting: Cell::new(false),
            request_draws: Default::default(),
        };
        Ok(this)
    }

    fn run_loop_inner(mut self) {
        let mut wm = self.window_manager.borrow_mut();
        wm.on_new_events(&self, StartCause::Init);
        wm.on_about_to_wait(&self);

        while !self.exiting.get() {
            // 0 for uninit
            let mut should_try_recv = Cell::new(0);

            let mut s_msg = None;
            let start_cause = match self.cf.get() {
                ControlFlow::Poll => {
                    if let Ok(msg) = self.recv.try_recv() {
                        s_msg = Some(msg);
                    }
                    StartCause::Poll
                }
                ControlFlow::Wait => {
                    let start = Instant::now();
                    let msg = self.recv.recv().expect("Failed to recv");
                    s_msg = Some(msg);
                    StartCause::WaitCancelled {
                        start,
                        requested_resume: Some(Instant::now()),
                    }
                }
                ControlFlow::WaitUntil(t) => {
                    let start = Instant::now();
                    if t > start {
                        match self.recv.recv_timeout(t.duration_since(start)) { Ok(msg) => {
                            s_msg = Some(msg);
                            StartCause::WaitCancelled {
                                start,
                                requested_resume: Some(Instant::now()),
                            }
                        } _ => {
                            StartCause::ResumeTimeReached {
                                start,
                                requested_resume: Instant::now(),
                            }
                        }}
                    } else {
                        match self.recv.try_recv() { Ok(msg) => {
                            s_msg = Some(msg);
                            StartCause::WaitCancelled {
                                start,
                                requested_resume: Some(Instant::now()),
                            }
                        } _ => {
                            StartCause::ResumeTimeReached {
                                start,
                                requested_resume: Instant::now(),
                            }
                        }}
                    }
                }
            };

            wm.on_new_events(&self, start_cause);

            let mut process_msg = |msg| match msg {
                LoopMessage::WindowEvent(id, e, t) => {
                    if matches!(e, WindowEvent::RedrawRequested) {
                        self.request_draws.borrow_mut().insert(id);
                    } else {
                        wm.on_window_event(&self, id, e, t);
                    }
                }
                LoopMessage::UserEvent(event) => {
                    wm.on_user_event(&self, event);
                }
                LoopMessage::NewLoop(l) => {
                    if should_try_recv.get() == 0 {
                        should_try_recv.set(1);
                    }
                }
                LoopMessage::Resumed => {
                    wm.on_resumed(&self);
                }
                LoopMessage::AboutToWait => {
                    should_try_recv.set(-1);
                }
            };
            if let Some(msg) = s_msg {
                process_msg(msg);
            }

            'l: while should_try_recv.get() == 1 {
                while let Ok(msg) = self.recv.try_recv() {
                    process_msg(msg);
                }
            }
            wm.on_about_to_wait(&self);
            self.request_draws.borrow().iter().for_each(|id| {
                wm.on_window_event(&self, *id, WindowEvent::RedrawRequested, Instant::now());
            });

            self.request_draws.borrow_mut().clear();
            log::trace!("Loop inner end");
            
        }
    }

    async fn run_loop_task<T: Send + 'static>(
        &self,
        f: Box<dyn FnOnce(&ActiveEventLoop) -> T + Send>,
    ) -> T {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = move |el: &'_ ActiveEventLoop| {
            let result = f(el);
            tx.send(result);
        };

        let box_task = Box::new(task);
        let msg = ToLoopMessage::RunTask(box_task);
        self.sender.send(msg);
        self.elp.send_event(WinitEventLoopMessage::CheckAsyncMsg);
        rx.await.unwrap()
    }
}

impl EngineEventLoopProxy for &AsyncWindowManagerInner {
    fn control_flow(&self) -> ControlFlow {
        self.cf.get()
    }

    fn exiting(&self) -> bool {
        self.exiting.get()
    }

    fn exit(&self) {
        self.exiting.set(true)
    }

    fn create_window(&self, attr: WindowAttributes) -> Result<Window, OsError> {
        block_on(AsyncWindowManagerInner::run_loop_task(
            self,
            Box::new(|el| el.create_window(attr)),
        ))
    }

    fn set_control_flow(&self, cf: ControlFlow) {
        self.cf.set(cf);
    }

    fn run_loop_task(&self, f: Box<dyn FnOnce(&ActiveEventLoop) + Send>) {
        block_on(AsyncWindowManagerInner::run_loop_task(
            self,
            Box::new(|el| f(el)),
        ))
    }

    fn request_draw(&self, window: &Window) {
        self.request_draws.borrow_mut().insert(window.id());
    }
}

pub trait EngineEventLoopProxyExt<'a, T> {
    fn run_loop_task_with_result(
        &'a self,
        f: Box<dyn FnOnce(&ActiveEventLoop) -> T + Send + 'a>,
    ) -> T;
}

impl<'a, T: EngineEventLoopProxy, R: Send + 'static> EngineEventLoopProxyExt<'a, R> for T {
    fn run_loop_task_with_result(
        &'a self,
        f: Box<dyn FnOnce(&ActiveEventLoop) -> R + Send + 'a>,
    ) -> R {
        // SAFETY: we await the result.
        let f = unsafe {
            std::mem::transmute::<_, Box<dyn FnOnce(&ActiveEventLoop) -> R + Send + 'static>>(f)
        };

        let (tx, rx) = futures::channel::oneshot::channel();
        let task = move |el: &'_ ActiveEventLoop| {
            let result = f(el);
            tx.send(result);
        };

        let box_task: Box<dyn FnOnce(&ActiveEventLoop) + Send> = Box::new(task);

        self.run_loop_task(box_task);
        block_on(rx).unwrap()
    }
}

pub struct AsyncWindowManager {
    /// The sender to run
    sender: SyncSender<LoopMessage>,
    rx: Receiver<ToLoopMessage>,
    handle: JoinHandle<()>,
}

struct RunMainThreadTask<T> {
    handle: RemoteHandle<T>,
}

impl<T: 'static> Future for RunMainThreadTask<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.handle.poll_unpin(cx)
    }
}

impl AsyncWindowManager {
    pub(crate) fn new(
        el: &EventLoop<WinitEventLoopMessage>,
        start: impl GameState + Send,
    ) -> anyhow::Result<Self> {
        let elp = el.create_proxy();
        let (tx, rx) = sync_channel(1024);
        let (ttx, trx) = channel();
        let handle = std::thread::Builder::new()
            .name("App Thread".to_string())
            .spawn(move || {
                let inner = AsyncWindowManagerInner::new(elp, rx, ttx, start).unwrap();
                inner.run_loop_inner();
            })
            .expect("Failed to create app thread");

        Ok(Self {
            sender: tx,
            rx: trx,
            handle,
        })
    }

    pub(crate) fn run_loop(mut self, event_loop: EventLoop<WinitEventLoopMessage>) {
        event_loop.listen_device_events(DeviceEvents::Never);
        let result = event_loop.run_app(&mut self);
        if let Err(e) = result {
            log::error!("Failed to run event loop for {:?}", e);
        }
    }
}

impl ApplicationHandler<WinitEventLoopMessage> for AsyncWindowManager {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if let Err(e) = self.sender.send(LoopMessage::NewLoop(cause)) {
            event_loop.exit();
        }
        log::trace!(target: "AWM", "New Event");
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(e) = self.sender.send(LoopMessage::Resumed) {
            event_loop.exit();
        }
        log::trace!(target: "AWM", "Resumed");

        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WinitEventLoopMessage) {
        log::trace!(target: "AWM", "User event");
        match event {
            WinitEventLoopMessage::WakeUp(w) => {
                if let Err(e) = self.sender.send(LoopMessage::UserEvent(event)) {
                    event_loop.exit();
                }
            }
            WinitEventLoopMessage::CheckAsyncMsg => {
                while let Ok(msg) = self.rx.try_recv() {
                    match msg {
                        ToLoopMessage::RunTask(task) => {
                            task(event_loop);
                        }
                    }
                }
            }
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        log::trace!(target: "AWM", "Window Event");
        let now = Instant::now();
        self.sender
            .send(LoopMessage::WindowEvent(window_id, event, now))
            .inspect_err(|_| event_loop.exit());
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        log::trace!(target: "AWM", "About to wait");

        self.sender
            .send(LoopMessage::AboutToWait)
            .inspect_err(|_| event_loop.exit());
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        //
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {}

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {}
}
