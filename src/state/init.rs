use std::sync::atomic::Ordering;
use std::sync::Arc;

use futures::task::SpawnExt;
use log::error;
use once_cell::sync::Lazy;
use wgpu::{Device, Queue};

use crate::engine::global::{INITED, IO_POOL, STATIC_DATA};
use crate::engine::{GameState, LoopState, ResourceManager, StateData, StateEvent, Trans, WaitFutureState, WaitResult};
use crate::game::song::SongManager;

pub struct InitState {
    start_state: Option<Box<dyn GameState + Send + 'static>>,
}

impl InitState {
    pub fn new(state: Box<dyn GameState + Send + 'static>) -> Self {
        Self {
            start_state: Some(state),
        }
    }
}


impl GameState for InitState {
    fn start(&mut self, s: &mut StateData) {}


    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        if let Some(gpu) = s.app.gpu.as_ref() {
            let state = self.start_state.take().unwrap();
            let device = gpu.device.clone();
            let queue = gpu.queue.clone();
            let res = s.app.res.clone();
            let handle = IO_POOL.spawn_with_handle(async move {
                let device = device;
                let queue = queue;
                let res = res;
                let task = async move {
                    if !INITED.load(Ordering::Acquire) {
                        Lazy::force(&STATIC_DATA);
                    }


                    anyhow::Ok(())
                };
                let song_manager = SongManager::init_manager()
                    .expect("Failed to init song manager");
                if let Err(e) = task.await {
                    error!("Load failed for {:?}", e);
                    WaitResult::Exit
                } else {
                    WaitResult::Function(Box::new(|s| {
                        s.app.egui_ctx.set_fonts(STATIC_DATA.font.clone());
                        s.wd.world.insert(Arc::new(song_manager));
                        Trans::Switch(state)
                    }))
                }
            }).expect("Spawn init task failed");


            (Trans::Push(WaitFutureState::from_wait_thing(handle)), LoopState::POLL_WITHOUT_RENDER)
        } else {
            (Trans::None, LoopState::WAIT_ALL)
        }
    }

    fn on_event(&mut self, s: &mut StateData, e: StateEvent) {
        if matches!(e, StateEvent::ReloadGPU) {
            let gpu = s.app.gpu.as_ref().expect("I FOUND GPU");
        }
    }
}
