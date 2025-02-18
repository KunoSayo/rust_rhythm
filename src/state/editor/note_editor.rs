use crate::engine::renderer::texture_renderer::TextureRenderer;
use crate::engine::{EguiExt, StateData};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::MapRule;
use crate::game::note::consts::NOTE_HEIGHT_PIXEL;
use crate::game::note::{LongNote, NormalNote, NoteHitType};
use crate::game::render::NoteRenderer;
use crate::game::OffsetType;
use crate::state::editor::editor::BeatMapEditor;
use crate::state::editor::util::map_point_to_std_pos_in_rect;
use egui::epaint::PathStroke;
use egui::panel::Side;
use egui::{Color32, Frame, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};
use num::Signed;
use std::collections::BTreeMap;
use std::ops::DerefMut;

#[derive(Default, Copy, Clone)]
pub struct ClickedPos {
    pub x: f32,
    pub time: OffsetType,
}

pub enum PointerType {
    Select(Option<ClickedPos>),
    NormalNote,
    LongNote(Option<ClickedPos>),
}

impl Default for PointerType {
    fn default() -> Self {
        Self::Select(None)
    }
}

pub struct BeatmapEditorData {
    /// The view seconds. At y = 1
    pub(crate) pointer_type: PointerType,
    pub normal_notes: BTreeMap<OffsetType, Vec<NormalNote>>,
    pub long_notes: BTreeMap<OffsetType, Vec<LongNote>>,
}


impl BeatmapEditorData {
    pub fn new(beatmap: &SongBeatmapFile) -> Self {
        let mut this = Self {
            pointer_type: Default::default(),
            normal_notes: Default::default(),
            long_notes: Default::default(),
        };

        for x in &beatmap.normal_notes {
            this.add_note(x)
        }
        for x in &beatmap.long_notes {
            this.add_long_note(&x);
        }

        this
    }

    pub fn add_note(&mut self, note: &NormalNote) {
        self.normal_notes
            .entry(note.time)
            .or_default()
            .push(note.clone());
    }
    pub fn add_long_note(&mut self, note: &LongNote) {
        self.long_notes
            .entry(note.start_time)
            .or_default()
            .push(note.clone());
    }

    pub fn contains_note(&self, note: &NormalNote) -> bool {
        if let Some(container) = self.normal_notes.get(&note.time) {
            return container.contains(&note);
        }
        false
    }

    pub fn contains_long_note(&self, note: &LongNote) -> bool {
        if let Some(container) = self.long_notes.get(&note.start_time) {
            return container.contains(&note);
        }
        false
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
    /// Return the (x, time)
    fn get_note_pos_for_cursor(&self, s: &mut StateData, game_rect: &Rect) -> (f32, OffsetType) {
        let mouse_pos = s.app.inputs.mouse_state.pos;

        let (mut x, y) =
            map_point_to_std_pos_in_rect(&game_rect, Pos2::new(mouse_pos.x, mouse_pos.y));
        let current_time = self.input_cache.current_duration.as_secs_f32();
        let view_secs = self.input_cache.progress_half_time;

        let end_time = current_time + view_secs;

        match self.beatmap.rule {
            MapRule::Falling => {}
            MapRule::FourKey => {
                x = get_nearest_result(x, &[-0.75, -0.25, 0.25, 0.75], None);
            }
        }

        let select_time = (((y * view_secs) + current_time) * 1000.0).round() as OffsetType;
        let mut times = vec![];
        self.beatmap
            .timing_group
            .get_beat_iterator(
                self.input_cache.select_timing_group,
                select_time,
                self.input_cache.detail,
            )
            .skip_while(|x| x.time < 0)
            .take_while(|x| x.time <= (end_time * 1000.0).round() as OffsetType)
            .for_each(|beat| {
                times.push(beat.time);
            });

        let result_time = get_nearest_result(select_time, &times, None);

        (x, result_time)
    }

    fn normal_note_pointer_update(&mut self, s: &mut StateData, ui: &mut Ui, game_rect: &Rect) {
        let pos = s.app.inputs.mouse_state.pos;
        if !game_rect.contains([pos.x, pos.y].into()) {
            return;
        }
        let place_note_pos = self.get_note_pos_for_cursor(s, game_rect);
        let note_center_y = self.time_map_y(place_note_pos.1 as f32 / 1000.0);
        let note_center_ui_y = self.time_map_ui_y(place_note_pos.1 as f32 / 1000.0, game_rect);
        let note_width = self.get_place_note_width();
        let note_ui_left_x = Self::game_x_map_ui_x(place_note_pos.0 - note_width * 0.5, game_rect);
        let note_ui_right_x = Self::game_x_map_ui_x(place_note_pos.0 + note_width * 0.5, game_rect);
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

        let note_to_place = NormalNote {
            x: place_note_pos.0,
            width: note_width,
            time: place_note_pos.1,
            note_type: NoteHitType::Click,
            timing_group: self.input_cache.select_timing_group as u8,
        };

        if s.app.inputs.mouse_state.take_is_clicked() {
            self.input_cache.edit_data.add_note(&note_to_place);
            self.dirty = true;
        }

        if !self.input_cache.edit_data.contains_note(&note_to_place) {
            let nr = s.app.world.get_mut::<NoteRenderer>().unwrap();
            nr.note_desc.get_note_render_obj(
                (game_rect.width(), game_rect.height()),
                self.input_cache.current_duration.as_secs_f32(),
                1.0 / self.input_cache.progress_half_time,
                &note_to_place,
                |obj| nr.background_objs.push(obj),
            );
        }

    }

    fn render_game_viewport(&mut self, s: &mut StateData, ui: &mut Ui, game_rect: &Rect) {
        let pos = s.app.inputs.mouse_state.pos;
        if game_rect.contains([pos.x, pos.y].into()) {
            self.scroll_beat(ui);
        }

        {
            let tr = s.app.world.fetch::<TextureRenderer>();
            let mut nr = s.app.world.fetch_mut::<NoteRenderer>();
            let current_time = self.input_cache.current_duration.as_secs_f32();
            let view_secs = self.input_cache.progress_half_time;

            let start_time = ((current_time - view_secs - 1.0) * 1000.0) as OffsetType;
            let end_time = (current_time + view_secs) * 1000.0;
            let end_time = end_time as OffsetType;
            let viewport_size = (game_rect.width(), game_rect.height());
            let center_time = self.input_cache.current_duration.as_secs_f32();
            let time_scale = 1.0 / self.input_cache.progress_half_time;

            let NoteRenderer {
                background_objs: bgs,
                note_desc: desc,
                objs: fgs,
                ..
            } = nr.deref_mut();
            let mut to_bg = |obj| {
                bgs.push(obj);
            };
            let mut to_fg = |obj| {
                fgs.push(obj);
            };
            for x in self.input_cache.edit_data.normal_notes.range(start_time..=end_time) {
                for note in x.1 {
                    if note.timing_group == self.input_cache.select_timing_group as u8 {
                        desc.get_note_render_obj(viewport_size, center_time, time_scale, note, &mut to_fg);
                    } else {
                        desc.get_note_render_obj(viewport_size, center_time, time_scale, note, &mut to_bg);
                    };
                }
            }
            for x in self.input_cache.edit_data.long_notes.range(start_time..=end_time) {

            }
        }

        match self.input_cache.edit_data.pointer_type {
            PointerType::Select(start_pos) => {}
            PointerType::NormalNote => {
                self.normal_note_pointer_update(s, ui, game_rect);
            }
            PointerType::LongNote(start_pos) => {}
        }



        let tr = s.app.world.fetch::<TextureRenderer>();
        let mut nr = s.app.world.fetch_mut::<NoteRenderer>();


        nr.render(
            s.app.gpu.as_ref().unwrap(),
            s.app.render.as_mut().unwrap(),
            &tr,
            &game_rect,
        );
    }

    pub fn render_note_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
        // First we need beautiful frame.
        egui::SidePanel::new(Side::Left, "note_left")
            .frame(Frame::NONE)
            .max_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let size = Vec2::new(100.0, 50.0);
                    let data = &mut self.input_cache.edit_data;
                    if ui
                        .select_button(
                            "Select",
                            matches!(data.pointer_type, PointerType::Select(_)),
                            size,
                        )
                        .clicked()
                    {
                        data.pointer_type = PointerType::Select(None);
                    }
                    if ui
                        .select_button(
                            "Normal",
                            matches!(data.pointer_type, PointerType::NormalNote),
                            size,
                        )
                        .clicked()
                    {
                        data.pointer_type = PointerType::NormalNote;
                    }
                    if ui
                        .select_button(
                            "Long",
                            matches!(data.pointer_type, PointerType::LongNote(_)),
                            size,
                        )
                        .clicked()
                    {
                        data.pointer_type = PointerType::LongNote(None);
                    }
                });
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
                    Stroke::new(5.0, Color32::from_rgba_unmultiplied(255, 255, 255, 127)),
                );

                self.render_game_viewport(s, ui, &rect);
            });
    }

    #[inline]
    #[must_use]
    fn time_map_y(&self, time: f32) -> f32 {
        (time - self.input_cache.current_duration.as_secs_f32())
            / self.input_cache.progress_half_time
    }
    #[inline]
    #[must_use]
    fn y_map_time(&self, y: f32) -> f32 {
        y * self.input_cache.progress_half_time + self.input_cache.current_duration.as_secs_f32()
    }

    #[inline]
    #[must_use]
    fn time_map_ui_y(&self, time: f32, rect: &Rect) -> f32 {
        rect.center().y - self.time_map_y(time) * rect.height() * 0.5
    }

    #[inline]
    #[must_use]
    fn ui_y_map_time(&self, y: f32, rect: &Rect) -> f32 {
        self.y_map_time((rect.center().y - y) / (rect.height() * 0.5))
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
}
