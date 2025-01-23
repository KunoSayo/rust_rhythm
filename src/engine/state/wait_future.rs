use std::future::Future;
use std::task::{Context, Poll, Waker};

use crate::engine::global::IO_POOL;
use crate::engine::prelude::*;
use crate::engine::task::wakers::WindowWaker;
use futures::future::RemoteHandle;
use futures::task::SpawnExt;
use futures::FutureExt;

#[allow(unused)]
/// The result after pop the wait state
pub enum WaitResult {
    Exit,
    Pop,
    Switch(Box<dyn GameState + Send + 'static>),
    Push(Box<dyn GameState + Send + 'static>),
    Function(Box<dyn FnOnce(&mut StateData) -> Trans + Send + 'static>),
}


/// The state will pop and execute the trans while the handle has result.
pub struct WaitFutureState {
    handle: Option<RemoteHandle<WaitResult>>,
    result: Option<WaitResult>,
    waker: Option<Waker>,
}


impl WaitFutureState {
    pub fn from_wait_thing(value: RemoteHandle<WaitResult>) -> Box<Self> {
        Self {
            handle: Some(value),
            result: None,
            waker: None,
        }.into()
    }

    pub fn wait_task(task: impl Future<Output=WaitResult> + Send + 'static) -> Box<Self> {
        let handle = IO_POOL.spawn_with_handle(task)
            .expect("Failed to spawn");
        Self::from_wait_thing(handle)
    }
}

impl WaitFutureState {
    fn check_result(&mut self) {
        if let Some(handle) = self.handle.as_mut() {
            let mut ctx = Context::from_waker(self.waker.as_ref().unwrap());
            match handle.poll_unpin(&mut ctx) {
                Poll::Ready(tran) => {
                    self.result = Some(tran);
                    self.handle.take();
                }
                Poll::Pending => {}
            }
        }
    }
}

impl GameState for WaitFutureState {
    fn start(&mut self, s: &mut StateData) -> LoopState {
        self.waker = Some(WindowWaker::new(s.wd.elp.clone(), &s.app.window).into());
        self.check_result();
        if self.result.is_some() {
            self.waker.take().unwrap().wake();
        }
        LoopState::WAIT_ALL
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        self.check_result();
        if let Some(thing) = self.result.take() {
            match thing {
                WaitResult::Function(f) => {
                    (Trans::Vec(vec![Trans::Pop, f(s)]), LoopState::POLL)
                }
                WaitResult::Exit => {
                    (Trans::Exit, LoopState::POLL)
                }
                WaitResult::Switch(s) => {
                    (Trans::Vec(vec![Trans::Pop, Trans::Switch(s)]), LoopState::POLL)
                }
                WaitResult::Pop => {
                    (Trans::Vec(vec![Trans::Pop, Trans::Pop]), LoopState::POLL)
                }
                WaitResult::Push(s) => {
                    (Trans::Switch(s), LoopState::POLL)
                }
            }
        } else {
            (Trans::None, LoopState::WAIT_ALL)
        }
    }
}

