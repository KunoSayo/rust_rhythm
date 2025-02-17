use crate::engine::StateData;
use crate::game::beatmap::MapRule;
use crate::game::note::consts::NOTE_HEIGHT_PIXEL;
use crate::game::OffsetType;
use crate::state::editor::editor::BeatMapEditor;
use crate::state::editor::util::map_point_to_std_pos_in_rect;
use egui::epaint::PathStroke;
use egui::panel::Side;
use egui::{Color32, Frame, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};
use num::Signed;

#[derive(Default)]
enum PointerType {
    #[default]
    Select,
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

fn get_nearest_result<T>(cur: T, ways: &[T], tolerance: Option<T>) -> T
where
    T: std::ops::Sub<Output = T> + Signed + Copy + PartialOrd,
{
    let mut min_dif = None;
    let mut result = cur;
    for x in ways {
        let cd = (*x - cur).abs();
        if let Some(t) = tolerance {
            if cd > t {
                continue;
            }
        }
        match min_dif {
            Some(md) => {
                if cd < md {
                    min_dif = Some(cd);
                    result = *x
                }
            }
            _ => {
                min_dif = Some(cd);
                result = *x;
            }
        }
    }

    result
}

impl BeatMapEditor {
    #[inline]
    #[must_use]
    fn time_map_y(&self, time: f32) -> f32 {
        (time - self.input_cache.current_duration.as_secs_f32())
            / self.input_cache.progress_half_time
    }
    #[inline]
    #[must_use]
    fn time_map_ui_y(&self, time: f32, rect: &Rect) -> f32 {
        rect.center().y - self.time_map_y(time) * rect.height() * 0.5
    }

    #[inline]
    #[must_use]
    fn game_x_map_ui_x(x: f32, rect: &Rect) -> f32 {
        rect.center().x + x * rect.width() * 0.5
    }

    #[inline]
    #[must_use]
    fn get_place_note_width(&self) -> f32 {
        match self.beatmap.rule {
            MapRule::Falling => self.input_cache.note_width,
            MapRule::FourKey => 0.25,
        }
    }

    /// Return the (x, time)
    fn get_note_pos_for_cursor(&self, s: &mut StateData, game_rect: &Rect) -> (f32, OffsetType) {
        let mouse_pos = s.app.inputs.mouse_state.pos;

        let (mut x, y) =
            map_point_to_std_pos_in_rect(&game_rect, Pos2::new(mouse_pos.x, mouse_pos.y));
        let current_time = self.input_cache.current_duration.as_secs_f32();
        let view_secs = self.input_cache.progress_half_time;

        match self.beatmap.rule {
            MapRule::Falling => {}
            MapRule::FourKey => {
                x = get_nearest_result(x, &[-0.75, -0.25, 0.25, 0.75], None);
            }
        }

        let select_time = (((y * view_secs) + current_time) * 1000.0).round() as OffsetType;
        // let mut times = vec![];
        let (left, now, right) = self.beatmap.timing_group.get_near_beat(
            self.input_cache.select_timing_group,
            select_time,
            self.input_cache.detail,
        );
        let mut result_time = select_time;
        if let Some(now) = now {
            result_time = now.time;
        } else {
            result_time = get_nearest_result(result_time, &[left.time, right.time], None);
        }

        (x, result_time)
    }

    fn render_game_viewport(&mut self, s: &mut StateData, ui: &mut Ui, game_rect: &Rect) {
        let pos = s.app.inputs.mouse_state.pos;
        if game_rect.contains([pos.x, pos.y].into()) {
            self.scroll_beat(ui);
            let place_note_pos = self.get_note_pos_for_cursor(s, game_rect);
            let note_center_ui_y = self.time_map_ui_y(place_note_pos.1 as f32 / 1000.0, game_rect);
            let note_width = self.get_place_note_width();
            let note_ui_left_x =
                Self::game_x_map_ui_x(place_note_pos.0 - note_width * 0.5, game_rect);
            let note_ui_right_x =
                Self::game_x_map_ui_x(place_note_pos.0 + note_width * 0.5, game_rect);
            let note_rect = Rect::from_min_max(
                Pos2::new(note_ui_left_x, note_center_ui_y - NOTE_HEIGHT_PIXEL * 0.5),
                Pos2::new(note_ui_right_x, note_center_ui_y + NOTE_HEIGHT_PIXEL * 0.5),
            );
            ui.painter().rect_stroke(
                note_rect,
                0.0,
                Stroke::new(1.0, Color32::YELLOW),
                StrokeKind::Outside,
            );
        }
    }

    pub fn render_note_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
        // First we need beautiful frame.
        egui::SidePanel::new(Side::Left, "note_left")
            .frame(Frame::NONE)
            .max_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {});
            });

        egui::CentralPanel::default()
            .frame(Frame::NONE)
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

                {
                    let rect = rect.expand(1.0);
                    let points = vec![
                        Pos2::new(rect.left(), rect.top()),
                        Pos2::new(rect.right(), rect.top()),
                        Pos2::new(rect.right(), rect.bottom()),
                        Pos2::new(rect.left(), rect.bottom()),
                        Pos2::new(rect.left(), rect.top()),
                    ];
                    ui.painter()
                        .line(points, PathStroke::new(1.0, Color32::WHITE));
                }

                ui.set_clip_rect(rect);

                // Render timing group && current time line
                for x in self.get_beat_iter(
                    self.input_cache.current_duration.as_secs_f32()
                        - self.input_cache.progress_half_time
                        - 1.0,
                ) {
                    if x.time as f32 / 1000.0
                        > self.input_cache.current_duration.as_secs_f32()
                            + self.input_cache.progress_half_time
                            + 1.0
                    {
                        break;
                    }
                    if x.time > self.total_duration.as_millis() as i64 {
                        break;
                    }
                    if x.time < 0 {
                        continue;
                    }
                    let time_y = self.time_map_ui_y(x.time as f32 / 1000.0, &rect);
                    let color = x.get_color();
                    if x.is_measure {
                        ui.painter().hline(
                            rect.left()..=rect.right(),
                            time_y,
                            Stroke::new(3.0, color),
                        );
                    } else {
                        ui.painter().hline(
                            rect.left()..=rect.right(),
                            time_y,
                            Stroke::new(1.0, color),
                        );
                    }
                }

                ui.painter().hline(
                    rect.left()..=rect.right(),
                    rect.center().y,
                    Stroke::new(5.0, Color32::WHITE),
                );

                self.render_game_viewport(s, ui, &rect);
            });
    }
}
