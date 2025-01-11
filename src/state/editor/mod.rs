mod util;
mod editor;

use crate::engine::{GameState, LoopState, StateData, Trans};
use egui::Context;

pub struct EditorMenu {}

impl EditorMenu {
    pub fn new() -> Self {
       Self {

       }
    }
}



impl GameState for EditorMenu {
    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        (Trans::None, LoopState::WAIT)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut tran = Trans::None;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.allocate_space((0.0, 100.0).into());

                if ui.button("Music | 音乐").clicked() {
                    if let Some(music) = util::select_music_file(&s.app.window) {

                    }
                }
            })
        });

        tran
    }
}