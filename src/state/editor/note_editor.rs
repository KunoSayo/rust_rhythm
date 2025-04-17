use crate::engine::renderer::texture_renderer::TextureRenderer;
use crate::engine::{EguiExt, StateData};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::{GamePos, MapRule, FOUR_KEY_X};
use crate::game::note::consts::NOTE_HEIGHT_PIXEL;
use crate::game::note::{LongNote, NormalNote, Note, NoteHitType};
use crate::game::render::NoteRenderer;
use crate::game::{get_play_rect, OffsetType};
use crate::state::editor::editor::BeatMapEditor;
use crate::state::editor::note_editor::PointerType::Select;
use crate::state::editor::util::map_point_to_std_pos_in_rect;
use egui::epaint::PathStroke;
use egui::panel::Side;
use egui::{Color32, Frame, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};
use num::Signed;
use std::collections::{BTreeMap, VecDeque};
use std::ops::DerefMut;
use winit::dpi::PhysicalPosition;
use winit::keyboard::{KeyCode, PhysicalKey};

pub enum SelectData {
    Clicking(GamePos),
    Selected(GamePos, GamePos, Vec<NormalNote>, Vec<LongNote>),
}

pub enum PointerType {
    Select(Option<SelectData>),
    NormalNote,
    LongNote(Option<GamePos>),
}

impl Default for PointerType {
    fn default() -> Self {
        Self::Select(None)
    }
}
#[derive(Copy, Clone)]
pub enum EditOps {
    Add,
    Del,
}

#[non_exhaustive]
#[derive(Clone)]
pub enum EditCommand {
    EditNote(EditOps, Vec<NormalNote>, Vec<LongNote>),
}

#[derive(Default)]
pub struct BeatmapEditorData {
    /// The view seconds. At y = 1
    pub(crate) cursor: PointerType,
    pub normal_notes: BTreeMap<OffsetType, Vec<NormalNote>>,
    pub long_notes: BTreeMap<OffsetType, Vec<LongNote>>,
    pub history: VecDeque<EditCommand>,
    pub undo_history: Vec<EditCommand>,
}

impl BeatmapEditorData {
    pub fn get_notes(&self, a: GamePos, b: GamePos) -> (Vec<NormalNote>, Vec<LongNote>) {
        let left_x = a.x.min(b.x);
        let right_x = a.x.max(b.x);

        let early = a.time.min(b.time);
        let later = a.time.max(b.time);
        let mut nn = Vec::<NormalNote>::new();
        let mut ln = Vec::<LongNote>::new();
        for entry in self.normal_notes.range(early..=later) {
            nn.extend(
                entry
                    .1
                    .iter()
                    .filter(|note| (left_x..=right_x).contains(&note.x)),
            )
        }
        for entry in self.long_notes.range(early..=later) {
            ln.extend(
                entry
                    .1
                    .iter()
                    .filter(|note| (left_x..=right_x).contains(&note.x)),
            )
        }
        (nn, ln)
    }
    pub fn new(beatmap: &SongBeatmapFile) -> Self {
        let mut this = Self::default();
        this.add_notes(&beatmap.normal_notes);
        this.add_long_notes(&beatmap.long_notes);

        this
    }

    fn add_history(&mut self, command: EditCommand) {
        self.history.push_back(command);
        if self.history.len() > 10240 {
            self.history.pop_back();
        }
        self.undo_history.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(command) = self.history.pop_back() {
            self.undo_cmd_with_record(command);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(command) = self.undo_history.pop() {
            let mut cur_undo = vec![];
            std::mem::swap(&mut cur_undo, &mut self.undo_history);
            self.do_cmd_with_record(command);
            std::mem::swap(&mut cur_undo, &mut self.undo_history);

            true
        } else {
            false
        }
    }

    /// Do edit command and record it.
    pub fn do_cmd_with_record(&mut self, command: EditCommand) {
        match &command {
            EditCommand::EditNote(op, note, ln) => match op {
                EditOps::Add => {
                    self.add_notes(&note);
                    self.add_long_notes(&ln);
                    self.add_history(command);
                }
                EditOps::Del => {
                    self.remove_long_notes(&ln);
                    self.remove_notes(&note);
                    self.add_history(command);
                }
            },
        }
    }

    pub fn undo_cmd_with_record(&mut self, command: EditCommand) {
        match &command {
            EditCommand::EditNote(op, note, ln) => {
                match op {
                    EditOps::Add => {
                        // delete it.
                        self.remove_long_notes(&ln);
                        self.remove_notes(&note);
                    }
                    EditOps::Del => {
                        self.add_notes(&note);
                        self.add_long_notes(&ln);
                    }
                }
                self.undo_history.push(command);
            }
        }
    }

    pub fn add_notes(&mut self, notes: &[NormalNote]) {
        for note in notes {
            self.normal_notes
                .entry(note.time)
                .or_default()
                .push(note.clone());
        }
    }

    /// Return remove successful
    pub fn remove_notes(&mut self, notes: &[NormalNote]) -> bool {
        let mut result = false;
        for note in notes {
            result |= if let Some(notes) = self.normal_notes.get_mut(&note.time) {
                let before = notes.len();
                notes.retain(|x| x != note);
                let now = notes.len();

                if now == 0 {
                    self.normal_notes.remove(&note.time);
                }

                before != now
            } else {
                false
            };
        }
        result
    }

    pub fn remove_long_notes(&mut self, notes: &[LongNote]) -> bool {
        let mut result = false;
        for note in notes {
            result |= if let Some(notes) = self.long_notes.get_mut(&note.start_time) {
                let before = notes.len();
                notes.retain(|x| x != note);
                let now = notes.len();

                if now == 0 {
                    self.long_notes.remove(&note.start_time);
                }

                before != now
            } else {
                false
            };
        }
        result
    }

    pub fn add_long_notes(&mut self, notes: &[LongNote]) {
        for note in notes {
            self.long_notes
                .entry(note.start_time)
                .or_default()
                .push(note.clone());
        }
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

    pub fn normal_notes(&self) -> &BTreeMap<OffsetType, Vec<NormalNote>> {
        &self.normal_notes
    }

    pub fn long_notes(&self) -> &BTreeMap<OffsetType, Vec<LongNote>> {
        &self.long_notes
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
    fn get_notes_from_click(
        &self,
        game_rect: &Rect,
        clicked_pos: GamePos,
        mouse_pos: PhysicalPosition<f32>,
    ) -> (Vec<NormalNote>, Vec<LongNote>) {
        let mut nn = Vec::<NormalNote>::new();
        let mut ln = Vec::<LongNote>::new();

        let pos = Pos2::new(mouse_pos.x, mouse_pos.y);
        for x in self
            .input_cache
            .edit_data
            .normal_notes()
            .range(clicked_pos.time - 1000..=clicked_pos.time + 1000)
        {
            nn.extend(
                x.1.iter()
                    .filter(|note| self.get_note_rect(game_rect, *note).contains(pos)),
            );
        }

        for x in self
            .input_cache
            .edit_data
            .long_notes()
            .range(clicked_pos.time - 1000..=clicked_pos.time + 1000)
        {
            ln.extend(
                x.1.iter()
                    .filter(|note| self.get_note_rect(game_rect, *note).contains(pos)),
            );
        }

        (nn, ln)
    }

    /// Return the note rect in ui coord.
    fn get_note_rect(&self, game_rect: &Rect, note: &impl Note) -> Rect {
        let x = note.get_x();
        let note_width = note.get_width();
        let time = note.get_time();

        let note_center_ui_y = self.time_map_ui_y(time as f32 / 1000.0, game_rect);
        let note_ui_left_x = Self::game_x_map_ui_x(x - note_width * 0.5, game_rect);
        let note_ui_right_x = Self::game_x_map_ui_x(x + note_width * 0.5, game_rect);

        let note_rect = if let Some(et) = note.get_end_time() {
            let note_end_center_ui_y = self.time_map_ui_y(et as f32 / 1000.0, game_rect);

            Rect::from_min_max(
                Pos2::new(note_ui_left_x, note_end_center_ui_y - NOTE_HEIGHT_PIXEL * 0.5),
                Pos2::new(
                    note_ui_right_x,
                    note_center_ui_y + NOTE_HEIGHT_PIXEL * 0.5,
                ),
            )
        } else {
            Rect::from_min_max(
                Pos2::new(note_ui_left_x, note_center_ui_y - NOTE_HEIGHT_PIXEL * 0.5),
                Pos2::new(note_ui_right_x, note_center_ui_y + NOTE_HEIGHT_PIXEL * 0.5),
            )
        };
        note_rect
    }
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
                x = get_nearest_result(x, &FOUR_KEY_X, None);
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

    /// Get the game pos from mouse pos.
    pub fn get_game_pos(&self, mouse_pos: PhysicalPosition<f32>, game_rect: &Rect) -> GamePos {
        let (x, y) = map_point_to_std_pos_in_rect(&game_rect, Pos2::new(mouse_pos.x, mouse_pos.y));
        let current_time = self.input_cache.current_duration.as_secs_f32();
        let view_secs = self.input_cache.progress_half_time;
        let select_time = (((y * view_secs) + current_time) * 1000.0).round() as OffsetType;

        GamePos::new(x, select_time)
    }

    fn highlight_note(&self, game_rect: &Rect, ui: &mut Ui, note: &impl Note) {
        let note_rect = self.get_note_rect(game_rect, note);
        ui.painter().rect_stroke(
            note_rect,
            0.0,
            Stroke::new(5.0, Color32::YELLOW),
            StrokeKind::Outside,
        );
    }

    fn select_cursor_render(&mut self, s: &mut StateData, ui: &mut Ui, game_rect: &Rect) {
        if let Select(data) = &self.input_cache.edit_data.cursor {
            let mut check_click_start = |this: &mut BeatMapEditor| {
                if this.allow_update {
                    if s.app.inputs.mouse_state.take_is_clicked() {
                        let pos = this.get_game_pos(s.app.inputs.mouse_state.pos, &game_rect);
                        this.input_cache.edit_data.cursor = Select(Some(SelectData::Clicking(pos)));
                    }
                }
            };
            if let Some(pointer_data) = data {
                match pointer_data {
                    SelectData::Clicking(start_pos) => {
                        if self.allow_update {
                            if !s.app.inputs.mouse_state.left_click {
                                let pos_now = s.app.inputs.mouse_state.pos;
                                let end_pos = self.get_game_pos(pos_now, &game_rect);
                                let (nn, ln) = if start_pos == &end_pos {
                                    self.get_notes_from_click(game_rect, *start_pos, pos_now)
                                } else {
                                    self.input_cache.edit_data.get_notes(*start_pos, end_pos)
                                };
                                self.input_cache.edit_data.cursor =
                                    Select(Some(SelectData::Selected(*start_pos, end_pos, nn, ln)));
                            }
                        }
                    }
                    SelectData::Selected(_, _, _, _) => check_click_start(self),
                }
            } else {
                // we have no click yet.
                check_click_start(self)
            }
        }

        // Render.
        if let Select(data) = &self.input_cache.edit_data.cursor {
            if let Some(data) = data {
                let mut draw_notes = |this: &Self, nn: &[NormalNote], ln: &[LongNote]| {
                    nn.iter().for_each(|note| {
                        this.highlight_note(&game_rect, ui, note);
                    });
                    ln.iter().for_each(|note| {
                        this.highlight_note(&game_rect, ui, note);
                    })
                };
                match data {
                    SelectData::Clicking(start) => {
                        let mouse_pos = s.app.inputs.mouse_state.pos;
                        let cur_pos = self.get_game_pos(mouse_pos, &game_rect);
                        let (nn, ln) = self.input_cache.edit_data.get_notes(*start, cur_pos);
                        draw_notes(self, &nn, &ln);

                        let start = self.game_pos_map_ui_pos(*start, game_rect);
                        let select_rect =
                            Rect::from_two_pos(start, Pos2::new(mouse_pos.x, mouse_pos.y));
                        ui.painter().rect_filled(
                            select_rect,
                            0.0,
                            Color32::from_rgba_unmultiplied(128, 128, 128, 128),
                        );
                        ui.painter().rect_stroke(
                            select_rect,
                            0.0,
                            Stroke::new(3.0, Color32::WHITE),
                            StrokeKind::Middle,
                        );
                    }
                    SelectData::Selected(_, _, nn, ln) => {
                        draw_notes(self, nn, ln);
                    }
                }
            }
        }
    }

    fn collect_background_note(&self, note: &impl Note, game_rect: &Rect, nr: &mut NoteRenderer) {
        nr.note_desc.get_note_render_obj(
            (game_rect.width(), game_rect.height()),
            self.input_cache.current_duration.as_secs_f32(),
            1.0 / self.input_cache.progress_half_time,
            note,
            |obj| nr.background_objs.push(obj),
        );
    }

    fn normal_note_pointer_update(&mut self, s: &mut StateData, ui: &mut Ui, game_rect: &Rect) {
        let pos = s.app.inputs.mouse_state.pos;
        if !game_rect.contains([pos.x, pos.y].into()) {
            return;
        }
        let place_note_pos = self.get_note_pos_for_cursor(s, game_rect);
        let note_width = self.get_place_note_width();

        let note_to_place = NormalNote {
            x: place_note_pos.0,
            width: note_width,
            time: place_note_pos.1,
            note_type: NoteHitType::Click,
            timing_group: self.input_cache.select_timing_group as u8,
        };

        if !self.input_cache.edit_data.contains_note(&note_to_place) {
            if self.allow_update {
                if s.app.inputs.mouse_state.take_is_clicked() {
                    self.input_cache
                        .edit_data
                        .do_cmd_with_record(EditCommand::EditNote(
                            EditOps::Add,
                            vec![note_to_place],
                            vec![],
                        ));
                    self.dirty = true;
                }
                let nr = s.app.world.get_mut::<NoteRenderer>().unwrap();
                self.collect_background_note(&note_to_place, game_rect, nr);
            }
        }
    }

    /// Function to update cursor, render the situation.
    fn long_note_pointer_update(
        &mut self,
        s: &mut StateData,
        ui: &mut Ui,
        game_rect: &Rect,
        start_pos: Option<GamePos>,
    ) {
        let pos = s.app.inputs.mouse_state.pos;
        if !game_rect.contains([pos.x, pos.y].into()) && start_pos.is_none() {
            return;
        }
        let place_note_pos = self.get_note_pos_for_cursor(s, game_rect);
        let start_pos = start_pos.unwrap_or(GamePos::new(place_note_pos.0, place_note_pos.1));
        let note_width = self.get_place_note_width();

        let note_to_place = LongNote {
            x: start_pos.x,
            width: note_width,
            start_time: start_pos.time.min(place_note_pos.1),
            timing_group: self.input_cache.select_timing_group as u8,
            end_time: place_note_pos.1.max(start_pos.time),
        };

        if !self
            .input_cache
            .edit_data
            .contains_long_note(&note_to_place)
        {
            if self.allow_update {
                match self.input_cache.edit_data.cursor {
                    PointerType::LongNote(None) => {
                        if s.app.inputs.mouse_state.left_click {
                            self.input_cache.edit_data.cursor =
                                PointerType::LongNote(Some(start_pos));
                        }
                    }
                    PointerType::LongNote(Some(start_pos)) => {
                        if s.app.inputs.mouse_state.is_released() {
                            if note_to_place.start_time != note_to_place.end_time {
                                self.input_cache.edit_data.do_cmd_with_record(
                                    EditCommand::EditNote(
                                        EditOps::Add,
                                        vec![],
                                        vec![note_to_place],
                                    ),
                                );
                            }
                            self.dirty = true;
                        }
                        if !s.app.inputs.mouse_state.left_click {
                            self.input_cache.edit_data.cursor = PointerType::LongNote(None);
                        }
                    }
                    _ => {}
                }
            }
            let nr = s.app.world.get_mut::<NoteRenderer>().unwrap();
            self.collect_background_note(&note_to_place, game_rect, nr);
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
            for x in self
                .input_cache
                .edit_data
                .normal_notes
                .range(start_time..=end_time)
            {
                for note in x.1 {
                    if note.timing_group == self.input_cache.select_timing_group as u8 {
                        desc.get_note_render_obj(
                            viewport_size,
                            center_time,
                            time_scale,
                            note,
                            &mut to_fg,
                        );
                    } else {
                        desc.get_note_render_obj(
                            viewport_size,
                            center_time,
                            time_scale,
                            note,
                            &mut to_bg,
                        );
                    };
                }
            }
            for x in self
                .input_cache
                .edit_data
                .long_notes
                .range(start_time..=end_time)
            {
                for note in x.1 {
                    if note.timing_group == self.input_cache.select_timing_group as u8 {
                        desc.get_note_render_obj(
                            viewport_size,
                            center_time,
                            time_scale,
                            note,
                            &mut to_fg,
                        );
                    } else {
                        desc.get_note_render_obj(
                            viewport_size,
                            center_time,
                            time_scale,
                            note,
                            &mut to_bg,
                        );
                    };
                }
            }
        }

        match &self.input_cache.edit_data.cursor {
            PointerType::Select(start_pos) => {
                self.select_cursor_render(s, ui, game_rect);
            }
            PointerType::NormalNote => {
                self.normal_note_pointer_update(s, ui, game_rect);
            }
            PointerType::LongNote(start_pos) => {
                self.long_note_pointer_update(s, ui, game_rect, start_pos.clone());
            }
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

    pub fn update_note_editor(&mut self, s: &mut StateData) {
        if s.app
            .inputs
            .is_pressed(&[PhysicalKey::Code(KeyCode::Delete)])
        {
            if let Select(Some(SelectData::Selected(start, end, nn, ln))) =
                &self.input_cache.edit_data.cursor
            {
                if nn.len() + ln.len() > 0 {
                    let start = *start;
                    let end = *end;
                    self.input_cache
                        .edit_data
                        .do_cmd_with_record(EditCommand::EditNote(
                            EditOps::Del,
                            nn.clone(),
                            ln.clone(),
                        ));
                    self.input_cache.edit_data.cursor =
                        Select(Some(SelectData::Selected(start, end, vec![], vec![])));
                    self.dirty = true;
                }
            }
        }

        if s.app.inputs.is_pressed(&[
            PhysicalKey::Code(KeyCode::ControlLeft),
            PhysicalKey::Code(KeyCode::KeyZ),
            PhysicalKey::Code(KeyCode::ShiftLeft),
        ]) {
            self.input_cache.edit_data.redo();
            self.dirty = true;
        } else if s.app.inputs.is_pressed(&[
            PhysicalKey::Code(KeyCode::ControlLeft),
            PhysicalKey::Code(KeyCode::KeyZ),
        ]) {
            self.input_cache.edit_data.undo();
            self.dirty = true;
        }
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
                            matches!(data.cursor, PointerType::Select(_)),
                            size,
                        )
                        .clicked()
                    {
                        data.cursor = PointerType::Select(None);
                    }
                    if ui
                        .select_button(
                            "Normal",
                            matches!(data.cursor, PointerType::NormalNote),
                            size,
                        )
                        .clicked()
                    {
                        data.cursor = PointerType::NormalNote;
                    }
                    if ui
                        .select_button(
                            "Long",
                            matches!(data.cursor, PointerType::LongNote(_)),
                            size,
                        )
                        .clicked()
                    {
                        data.cursor = PointerType::LongNote(None);
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let center_point = rect.center();
                // 4:3 current

                let rect = get_play_rect(rect);

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
        // up y is small.
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
    fn game_pos_map_ui_pos(&self, game_pos: GamePos, game_rect: &Rect) -> Pos2 {
        Pos2::new(
            Self::game_x_map_ui_x(game_pos.x, game_rect),
            self.time_map_ui_y(game_pos.time as f32 / 1000.0, game_rect),
        )
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
