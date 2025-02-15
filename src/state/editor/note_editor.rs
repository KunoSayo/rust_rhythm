use crate::engine::StateData;
use crate::state::editor::editor::BeatMapEditor;
use egui::epaint::PathStroke;
use egui::panel::Side;
use egui::{Color32, Frame, Pos2, Rect, Vec2};


#[derive(Default)]
enum PointerType {
    #[default]
    Select
}

pub struct BeatmapEditorData {
    /// The view seconds. At y = 1
    pointer_type: PointerType,
}

impl Default for BeatmapEditorData {
    fn default() -> Self {
        Self {
            pointer_type: Default::default(),
        }
    }
}


impl BeatMapEditor {
    pub fn render_note_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
        // First we need beautiful frame.
        egui::SidePanel::new(Side::Left, "note_left")
            .frame(Frame::none())
            .max_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {});
            });

        let current_time = self.input_cache.current_duration.as_secs_f32();
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


                let time_map_y = |time: f32| {
                    (time - current_time) / self.input_cache.progress_half_time
                };

                let time_map_ui_y = |time: f32| {
                    rect.center().y - time_map_y(time) * rect.height() * 0.5
                };


                let points = vec![Pos2::new(rect.left(), rect.top()),
                                  Pos2::new(rect.right(), rect.top()),
                                  Pos2::new(rect.right(), rect.bottom()),
                                  Pos2::new(rect.left(), rect.bottom()),
                                  Pos2::new(rect.left(), rect.top())];
                ui.painter().line(points, PathStroke::new(1.0, Color32::WHITE));
                
                ui.set_clip_rect(rect);

                // Render timing group && current time line
                for x in self.get_beat_iter(self.input_cache.current_duration.as_secs_f32() - self.input_cache.progress_half_time - 1.0) {
                    if x.time as f32 / 1000.0 > self.input_cache.current_duration.as_secs_f32() + self.input_cache.progress_half_time + 1.0 {
                        break;
                    }
                    if x.time > self.total_duration.as_millis() as i64 {
                        break;
                    }
                    if x.time < 0 {
                        continue;
                    }
                    let time_y = time_map_ui_y(x.time as f32 / 1000.0);
                    if x.is_measure {
                        ui.painter().hline(rect.left()..=rect.right(), time_y, PathStroke::new(3.0, Color32::from_gray(233)));
                    } else {
                        ui.painter().hline(rect.left()..=rect.right(), time_y, PathStroke::new(1.0, Color32::from_gray(233)));
                    }
                }

                ui.painter().hline(rect.left()..=rect.right(), rect.center().y, PathStroke::new(5.0, Color32::WHITE));
            });
    }
}