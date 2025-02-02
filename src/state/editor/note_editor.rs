use crate::engine::StateData;
use crate::state::editor::editor::BeatMapEditor;
use egui::epaint::PathStroke;
use egui::panel::Side;
use egui::{Color32, Frame, Pos2, Rect, Vec2};

impl BeatMapEditor {
    pub fn render_note_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
        // First we need beautiful frame.

        egui::SidePanel::new(Side::Left, "note_left")
            .frame(Frame::none())
            .max_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {});

        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let center_point = rect.center();
                // 4:3 current
                let (half_x, half_y) = if rect.height() <= rect.width() {
                    // expand to the top
                    let half_y = rect.height() / 2.0 - 10.0;
                    let half_x = half_y * 4.0 / 3.0;
                    (half_x, half_y)
                } else {
                    // expand to the left
                    let half_x = rect.width() / 2.0 - 10.0;
                    let half_y = half_x * 0.75;
                    (half_x, half_y)
                };

                let rect = Rect {
                    min: center_point - Vec2::new(half_x, half_y),
                    max: center_point + Vec2::new(half_x, half_y),
                };

                let points = vec![Pos2::new(rect.left(), rect.top()),
                                  Pos2::new(rect.right(), rect.top()),
                                  Pos2::new(rect.right(), rect.bottom()),
                                  Pos2::new(rect.left(), rect.bottom()),
                                  Pos2::new(rect.left(), rect.top())];
                ui.painter().line(points, PathStroke::new(1.0, Color32::WHITE));
            });
    }
}