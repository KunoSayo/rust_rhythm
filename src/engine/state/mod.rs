#![allow(unused)]


use std::cell::RefCell;
use std::time::{Duration, Instant};

use egui::epaint::ahash::HashMap;
use egui::NumExt;
use specs::World;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

pub use wait_future::*;

use crate::engine::app::AppInstance;
use crate::engine::manager::{EngineEventLoopProxy, EventLoopProxyType, EventLoopTargetType, WindowInstance};

mod wait_future;

pub unsafe fn cast_static<'a, T>(x: &'a T) -> &'static T { unsafe {
    std::mem::transmute(x)
}}

#[non_exhaustive]
pub enum Trans {
    None,
    Push(Box<dyn GameState>),
    Pop,
    Switch(Box<dyn GameState>),
    Exit,
    Vec(Vec<Trans>),
    IntoSwitch
}

#[derive(Debug, Copy, Clone)]
pub enum StateEvent<'a> {
    ReloadGPU,
    PostUiRender,
    Resume,
    Window(&'a WindowEvent, Instant),
}

impl Default for Trans {
    fn default() -> Self {
        Self::None
    }
}

pub struct GlobalData<'a> {
    pub el: &'a dyn EngineEventLoopProxy,
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
    fn start(&mut self, _: &mut StateData) -> LoopState { LoopState::WAIT_ALL }

    /// Update when event cleared
    fn update(&mut self, _: &mut StateData) -> (Trans, LoopState) { (Trans::None, LoopState::WAIT) }

    fn shadow_update(&mut self, _: &mut StateData) -> LoopState { LoopState::WAIT_ALL }

    /// Callback if render after the main event cleared
    /// GPU must be Some when calling this
    fn render(&mut self, _: &mut StateData, _: &egui::Context) -> Trans { Trans::None }

    fn shadow_render(&mut self, _: &mut StateData, _: &egui::Context) {}

    fn stop(&mut self, _: &mut StateData) {}

    fn on_event(&mut self, _: &mut StateData, _: StateEvent) {}
    
    fn switch(self: Box<Self>) -> Trans { Trans::None }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct LoopState {
    pub control_flow: ControlFlow,
    pub render: f32,
}

impl Default for LoopState {
    fn default() -> Self {
        Self {
            control_flow: ControlFlow::Poll,
            render: 1.0,
        }
    }
}

impl LoopState {
    #[allow(unused)]
    pub const WAIT_ALL: LoopState = LoopState {
        control_flow: ControlFlow::Wait,
        render: 0.0,
    };

    #[allow(unused)]
    pub const WAIT: LoopState = LoopState {
        control_flow: ControlFlow::Wait,
        render: 1.0,
    };

    #[allow(unused)]
    pub const POLL: LoopState = LoopState {
        control_flow: ControlFlow::Poll,
        render: 1.0,
    };

    #[allow(unused)]
    pub const POLL_WITHOUT_RENDER: LoopState = LoopState {
        control_flow: ControlFlow::Poll,
        render: 0.0,
    };

    #[allow(unused)]
    pub fn wait_until(dur: Duration, render: f32) -> Self {
        Self {
            control_flow: ControlFlow::WaitUntil(std::time::Instant::now() + dur),
            render,
        }
    }

    /// Reset the case and minus the render time
    pub fn reset(&mut self, dt: f32) {
        self.control_flow = ControlFlow::Wait;
        self.render = (self.render - dt).at_least(0.0);
    }
}

impl GameState for () {}

impl std::ops::BitOrAssign for LoopState {
    fn bitor_assign(&mut self, rhs: Self) {
        self.render = self.render.max(rhs.render);
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
