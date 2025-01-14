#![allow(unused)]


use std::cell::RefCell;
use std::time::Duration;

use egui::epaint::ahash::HashMap;
use specs::World;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

pub use wait_future::*;

use crate::engine::app::AppInstance;
use crate::engine::window::{EventLoopProxyType, EventLoopTargetType, WindowInstance};

mod wait_future;

pub unsafe fn cast_static<'a, T>(x: &'a T) -> &'static T {
    std::mem::transmute(x)
}

pub enum Trans {
    None,
    Push(Box<dyn GameState>),
    Pop,
    Switch(Box<dyn GameState>),
    Exit,
    Vec(Vec<Trans>),
}

#[derive(Debug, Copy, Clone)]
pub enum StateEvent<'a> {
    ReloadGPU,
    PostUiRender,
    Window(&'a WindowEvent),
}

impl Default for Trans {
    fn default() -> Self {
        Self::None
    }
}

pub struct GlobalData<'a> {
    pub el: &'a EventLoopTargetType,
    pub elp: &'a EventLoopProxyType,
    pub windows: &'a HashMap<WindowId, RefCell<Box<WindowInstance>>>,
    pub new_windows: &'a mut Vec<WindowInstance>,
    pub world: &'a mut World,
}


pub struct StateData<'a, 'b, 'c> {
    pub app: &'a mut AppInstance,
    pub wd: &'b mut GlobalData<'c>,
    pub dt: f32,
}


pub trait GameState: 'static {
    fn start(&mut self, _: &mut StateData) {}

    /// Update when event cleared
    fn update(&mut self, _: &mut StateData) -> (Trans, LoopState) { (Trans::None, LoopState::WAIT) }

    fn shadow_update(&mut self, _: &mut StateData) -> LoopState { LoopState::WAIT_ALL }

    /// Callback if render after the main event cleared
    /// GPU must be Some when calling this
    fn render(&mut self, _: &mut StateData, _: &egui::Context) -> Trans { Trans::None }

    fn shadow_render(&mut self, _: &mut StateData, _: &egui::Context) {}

    fn stop(&mut self, _: &mut StateData) {}

    fn on_event(&mut self, _: &mut StateData, _: StateEvent) {}
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct LoopState {
    pub control_flow: ControlFlow,
    pub render: bool,
}


impl Default for LoopState {
    fn default() -> Self {
        Self {
            control_flow: ControlFlow::Poll,
            render: true,
        }
    }
}

impl LoopState {
    #[allow(unused)]
    pub const WAIT_ALL: LoopState = LoopState {
        control_flow: ControlFlow::Wait,
        render: false,
    };

    #[allow(unused)]
    pub const WAIT: LoopState = LoopState {
        control_flow: ControlFlow::Wait,
        render: true,
    };

    #[allow(unused)]
    pub const POLL: LoopState = LoopState {
        control_flow: ControlFlow::Poll,
        render: true,
    };

    #[allow(unused)]
    pub const POLL_WITHOUT_RENDER: LoopState = LoopState {
        control_flow: ControlFlow::Poll,
        render: false,
    };

    #[allow(unused)]
    pub fn wait_until(dur: Duration, render: bool) -> Self {
        Self {
            control_flow: ControlFlow::WaitUntil(std::time::Instant::now() + dur),
            render,
        }
    }
}

impl GameState for () {}

impl std::ops::BitOrAssign for LoopState {
    fn bitor_assign(&mut self, rhs: Self) {
        self.render |= rhs.render;
        if self.control_flow != rhs.control_flow {
            match self.control_flow {
                ControlFlow::Wait => self.control_flow = rhs.control_flow,
                ControlFlow::WaitUntil(t1) => match rhs.control_flow {
                    ControlFlow::Wait => {}
                    ControlFlow::WaitUntil(t2) => {
                        self.control_flow = ControlFlow::WaitUntil(t1.min(t2));
                    }
                    _ => {
                        self.control_flow = rhs.control_flow;
                    }
                },

                _ => {}
            }
        }
    }
}


#[cfg(test)]
mod test {
    use crate::engine::LoopState;

    #[test]
    fn loop_state_test() {
        let mut s = LoopState::WAIT_ALL;
        s |= LoopState::WAIT;
        assert_eq!(s, LoopState::WAIT);
        s |= LoopState::POLL_WITHOUT_RENDER;

        // we already render
        assert_eq!(s, LoopState::POLL);
    }
}
