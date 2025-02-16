use std::time::Instant;
use crate::engine::{GameState, LoopState, StateData, StateEvent, Trans};
use crate::state::editor::EditorMenu;
use egui::{Button, Context, Frame, Widget};
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct MenuState {
    show_debug: bool,
}

impl MenuState {
    pub fn new() -> Self {
        Self { show_debug: false }
    }
}


impl GameState for MenuState {
    fn start(&mut self, s: &mut StateData) -> LoopState {
        // Render first!
        LoopState::WAIT
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        (Trans::None, LoopState::WAIT)
    }

    fn shadow_update(&mut self, s: &mut StateData) -> LoopState {
        if s.app.inputs.is_pressed(&[PhysicalKey::Code(KeyCode::F3)]) {
            self.show_debug ^= true;
        }

        LoopState::WAIT_ALL
    }
    
    


    fn render(&mut self, sd: &mut StateData, ctx: &Context) -> Trans {
        let mut tran = Trans::None;
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
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

    fn shadow_render(&mut self, s: &mut StateData, ctx: &Context) {
        if self.show_debug {
            egui::CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    ui.label(format!("fps: {:.2}", 1.0 / s.dt))
                });
        }
    }


    fn on_event(&mut self, s: &mut StateData, e: StateEvent) {}
}
