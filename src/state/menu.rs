use crate::engine::{GameState, LoopState, StateData, StateEvent, Trans};

pub struct MenuState {
}

impl MenuState {
    pub fn new() -> Self {
        Self {
        }
    }
}


impl GameState for MenuState {
    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        (Trans::None, LoopState::WAIT)
    }

    fn on_event(&mut self, s: &mut StateData, e: StateEvent) {

    }
}
