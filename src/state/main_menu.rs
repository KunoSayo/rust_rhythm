use crate::engine::{GameState, LoopState, StateData, StateEvent, Trans};
use crate::state::editor::EditorMenu;
use egui::{Button, Context, Widget};
use rayon::prelude::*;

pub struct MenuState {}

impl MenuState {
    pub fn new() -> Self {
        Self {}
    }
}


impl GameState for MenuState {
    fn start(&mut self, s: &mut StateData) {}

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        (Trans::None, LoopState::WAIT)
    }

    fn render(&mut self, sd: &mut StateData, ctx: &Context) -> Trans {
        let mut tran = Trans::None;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let height = ui.available_height();
                let button_height = 100.0f32;
                let padding = ui.style().spacing.button_padding.y;

                let button_num = 2f32;
                let total_height = button_height * button_num + (button_num - 1f32) * padding;

                ui.allocate_space((0.0, (height - total_height).max(0.0) / 2.0).into());

                if Button::new("Play").min_size((200.0, 100.0).into()).ui(ui).clicked() {}

                if Button::new("Editor").min_size((200.0, 100.0).into()).ui(ui).clicked() {
                    tran = Trans::Push(Box::new(EditorMenu::new()));
                }
            });
        });

        tran
    }


    fn on_event(&mut self, s: &mut StateData, e: StateEvent) {}
}
