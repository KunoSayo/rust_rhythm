use crate::engine::{GameState, LoopState, StateData, Trans};
use egui::Context;
use winit::keyboard::{KeyCode, PhysicalKey};
use crate::game::beatmap::summary::BeatmapPlayResult;

pub struct EndResultState {
    pub result: BeatmapPlayResult
}


impl GameState for EndResultState {
    fn start(&mut self, s: &mut StateData) -> LoopState {
        LoopState::WAIT
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        let mut trans = Trans::None;
        let cur_input = &s.app.inputs.cur_frame_input;
        if cur_input
            .pressing
            .contains(&PhysicalKey::Code(KeyCode::Escape))
        {
            trans = Trans::Pop;
        }
        (trans, LoopState::WAIT)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut trans = Trans::None;

        trans
    }
}
