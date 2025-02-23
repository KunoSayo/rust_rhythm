use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::engine::atlas::TextureAtlas;
use crate::engine::global::{INITED, IO_POOL, STATIC_DATA};
use crate::engine::renderer::texture_renderer::TextureRenderer;
use crate::engine::{GameState, LoopState, ResourceLocation, ResourceManager, StateData, StateEvent, Trans, WaitFutureState, WaitResult};
use crate::game::song::SongManager;
use futures::task::SpawnExt;
use log::error;
use once_cell::sync::Lazy;
use wgpu::{Device, Queue, ShaderModule};
use crate::game::render::NoteRenderer;

pub struct InitState {
    start_state: Option<Box<dyn GameState + Send + 'static>>,
}

impl InitState {
    pub fn new(state: Box<dyn GameState + Send + 'static>) -> Self {
        Self {
            start_state: Some(state),
        }
    }

    #[allow(unused)]
    pub async fn init_tasks(device: Arc<Device>, queue: Arc<Queue>, res: Arc<ResourceManager>) {
        let note = image::load_from_memory(&res.load_asset("texture/note.png").unwrap()).unwrap();

        let atlas = TextureAtlas::make_atlas(&device, &queue, &[(ResourceLocation::from_name("note"), &note)]).unwrap();
        res.atlas.insert(ResourceLocation::from_name("default"), atlas.into());
    }
}


impl GameState for InitState {
    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        match s.app.gpu.as_ref() { Some(gpu) => {
            let state = self.start_state.take().unwrap();
            let device = gpu.device.clone();
            let queue = gpu.queue.clone();
            let res = s.app.res.clone();
            s.app.world.insert(TextureRenderer::new(gpu));
            let handle = IO_POOL.spawn_with_handle(async move {
                let device = device;
                let queue = queue;
                let res = res;
                let task = async move {
                    if !INITED.load(Ordering::Acquire) {
                        Lazy::force(&STATIC_DATA);
                    }

                    Self::init_tasks(device, queue, res).await;

                    anyhow::Ok(())
                };
                let song_manager = SongManager::init_manager()
                    .expect("Failed to init song manager");
                match task.await { Err(e) => {
                    error!("Load failed for {:?}", e);
                    WaitResult::Exit
                } _ => {
                    WaitResult::Function(Box::new(|s| {
                        s.app.egui_ctx.set_fonts(STATIC_DATA.font.clone());
                        s.wd.world.insert(Arc::new(song_manager));
                        let gpu = s.app.gpu.as_ref().unwrap();
                        let tr = s.app.world.get_mut::<TextureRenderer>()
                            .unwrap();
                        let nr = NoteRenderer::new(&s.app.gpu.as_ref().unwrap().device, &tr, &s.app.res);
                        s.app.world.insert(nr);
                        Trans::Switch(state)
                    }))
                }}
            }).expect("Spawn init task failed");


            (Trans::Push(WaitFutureState::from_wait_thing(handle)), LoopState::POLL)
        } _ => {
            (Trans::None, LoopState::WAIT)
        }}
    }

    fn on_event(&mut self, s: &mut StateData, e: StateEvent) {
        if matches!(e, StateEvent::ReloadGPU) {
            let gpu = s.app.gpu.as_ref().expect("I FOUND GPU");
        }
    }
}
