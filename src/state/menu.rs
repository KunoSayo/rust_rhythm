use crate::engine::{GameState, LoopState, StateData, StateEvent, Trans};
use egui::epaint::PathStroke;
use egui::{Context, Pos2};

pub struct MenuState {}

impl MenuState {
    pub fn new() -> Self {
        Self {}
    }
}


impl GameState for MenuState {
    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        (Trans::None, LoopState::WAIT)
    }

    fn render(&mut self, sd: &mut StateData, ctx: &Context) -> Trans {
        egui::Window::new("Menu")
            .frame(egui::Frame::none())
            .resizable(true)
            .default_size((400.0, 400.0))
            .max_size((1600.0, 900.0))
            .show(ctx, |ui| {
                ui.label("Test line:");
                ui.allocate_space(ui.available_size());
                ui.painter()
                    .line_segment([Pos2::new(5.0, 5.0), Pos2::new(1600.0, 100.0)],
                                  PathStroke::new(3.0, egui::Color32::WHITE));
            });

        Trans::None
    }


    fn on_event(&mut self, s: &mut StateData, e: StateEvent) {}
}
