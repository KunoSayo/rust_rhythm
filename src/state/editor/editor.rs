use crate::engine::global::{IO_POOL, STATIC_DATA};
use crate::engine::{get_edit_cache, sample_change_speed, GameState, LoopState, StateData, Trans};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::{SongBeatmapInfo, BEATMAP_EXT};
use crate::game::song::{SongInfo, SongManagerResourceType};
use crate::game::timing::TimingGroupBeatIterator;
use crate::game::{offset_type_to_secs, secs_to_offset_type, OffsetType};
use crate::state::editor::note_editor::{BeatmapEditorData, PointerType};
use anyhow::anyhow;
use egui::panel::TopBottomSide;
use egui::{
    Align, Button, Color32, Context, Frame, Layout, NumExt, Pos2, Rect, Sense, Stroke, TextEdit,
    TextStyle, Ui, UiBuilder, Vec2,
};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStreamHandle, Sink, Source};
use std::io::{Cursor, Read};
use std::ops::{Add, ControlFlow, Deref, Div, Mul};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI16, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct SongSampleInfo {
    samples: Vec<i16>,
    samples_q: Vec<i16>,
    samples_half: Vec<i16>,
    samples_t_f: Vec<i16>,
    sample_rate: u32,
    channels: u16,
}

impl SongSampleInfo {
    pub fn new(samples: Vec<i16>, rate: u32, channels: u16) -> Self {
        use rayon::prelude::*;
        let result = [0.25, 0.5, 0.75]
            .into_par_iter()
            .map(|x| sample_change_speed(&samples, channels as usize, x))
            .collect::<Vec<_>>();
        let mut it = result.into_iter();
        let samples_q = it.next().unwrap();
        let samples_half = it.next().unwrap();
        let samples_t_f = it.next().unwrap();
        Self {
            samples,
            samples_q,
            samples_half,
            samples_t_f,
            sample_rate: rate,
            channels,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SubEditor {
    Settings,
    Note,
    Timing,
}

pub struct BeatMapEditor {
    pub song_info: Arc<SongInfo>,
    pub beatmap: SongBeatmapFile,
    save_path: Option<PathBuf>,
    pub total_duration: Duration,
    sink: Sink,
    pub(in crate::state::editor) input_cache: InputCache,

    sample_info: SongSampleInfo,

    current_editor: SubEditor,
    pub dirty: bool,
    /// allow update by input this render, for we may skip update due to some cases.
    pub allow_update: bool,
    play_speed: f32,
}

pub(in crate::state::editor) struct InputCache {
    pub(in crate::state::editor) escape_time: f32,
    pub(in crate::state::editor) detail: u8,
    pub(in crate::state::editor) current_duration: Duration,
    pub(in crate::state::editor) progress_half_time: f32,
    pub(in crate::state::editor) select_timing_group: usize,
    pub(in crate::state::editor) select_timing_row: Option<usize>,
    pub(in crate::state::editor) edit_data: BeatmapEditorData,
    pub(in crate::state::editor) note_width: f32,
}

impl InputCache {
    fn new(beatmap: &SongBeatmapFile) -> Self {
        Self {
            escape_time: 0.0,
            detail: 1,
            current_duration: Default::default(),
            progress_half_time: 0.5,
            select_timing_group: 0,
            select_timing_row: None,
            edit_data: BeatmapEditorData::new(beatmap),
            note_width: 0.25,
        }
    }
}

impl BeatMapEditor {
    pub fn new(song_info: Arc<SongInfo>, handle: OutputStreamHandle) -> anyhow::Result<Self> {
        Self::with_file(song_info, None, handle)
    }

    pub fn with_file(
        song_info: Arc<SongInfo>,
        info: Option<SongBeatmapInfo>,
        s: OutputStreamHandle,
    ) -> anyhow::Result<Self> {
        let sink = Sink::try_new(&s).expect("Failed to new sink");

        let mut buf = vec![];
        let mut file = std::fs::File::open(&song_info.bgm_file)?;
        file.read_to_end(&mut buf)?;

        let buf = Cursor::new(buf);
        let decoder = Decoder::new(buf.clone())?;

        let samples = decoder.convert_samples::<f32>();

        let total_duration = samples
            .total_duration()
            .ok_or(anyhow!("No audio duration"))?;
        sink.pause();
        sink.append(samples);

        let vol = STATIC_DATA
            .cfg_data
            .write()
            .map_err(|e| anyhow!("Cannot read lock for {:?}", e))?
            .get_f32_def("bgm_vol", 1.0);
        sink.set_volume(vol);

        let path = info.as_ref().map(|x| x.file_path.clone());

        let sample_info = {
            let decoder = Decoder::new(buf.clone())?;

            let sample_rate = decoder.sample_rate();
            let channels = decoder.channels();
            let samples = decoder.convert_samples::<i16>().collect();
            SongSampleInfo::new(samples, sample_rate, channels)
        };

        let dirty = info.is_none();
        let current_editor = SubEditor::Timing;
        let beatmap = info
            .map(|x| x.song_beatmap_file)
            .unwrap_or(SongBeatmapFile::new(song_info.title.clone()));
        let input_cache = InputCache::new(&beatmap);
        Ok(Self {
            beatmap,
            song_info,
            sink,
            save_path: path,
            total_duration,
            input_cache,
            sample_info,
            current_editor,
            dirty,
            allow_update: false,
            play_speed: 1.0,
        })
    }

    pub fn save(&mut self, s: &mut StateData) {
        if self.save_path.is_none()
            && (self.beatmap.metadata.title.is_empty() || self.beatmap.metadata.version.is_empty())
        {
            return;
        }
        if !self.dirty {
            return;
        }
        let path = self.save_path.get_or_insert_with(|| {
            self.song_info.bgm_file.parent().unwrap().join(
                format!(
                    "{}[{}]",
                    &self.beatmap.metadata.title, &self.beatmap.metadata.version
                ) + "."
                    + BEATMAP_EXT,
            )
        });
        let path = path.clone();

        {
            // deal notes
            self.beatmap.normal_notes.clear();
            self.input_cache
                .edit_data
                .normal_notes
                .iter()
                .for_each(|x| self.beatmap.normal_notes.extend_from_slice(&x.1));

            self.beatmap.long_notes.clear();
            self.input_cache
                .edit_data
                .long_notes
                .iter()
                .for_each(|x| self.beatmap.long_notes.extend_from_slice(&x.1));
        }
        let beatmap = self.beatmap.clone();
        let info = self.song_info.clone();
        let song_manager =
            s.wd.world
                .fetch::<SongManagerResourceType>()
                .deref()
                .clone();
        // IO_POOL.spawn_ok(async move {
        match beatmap.save_to(&path) { Err(e) => {
            log::error!("Failed to save beatmap for {:?}", e);
        } _ => {
            match info.reload() {
                Ok(new_info) => {
                    song_manager.load_new_info(new_info);
                    info.dirty.store(true, Ordering::Release);
                    self.dirty = false;
                }
                Err(e) => {
                    log::error!("Failed to load beatmap for {:?}", e);
                }
            }
        }}
        // });
    }
}

impl GameState for BeatMapEditor {
    fn start(&mut self, s: &mut StateData) -> LoopState {
        s.app.window.set_title(&format!(
            "{}{}",
            &self.beatmap.metadata.title,
            ["", " *"][self.dirty as usize]
        ));

        LoopState::WAIT_ALL
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        self.allow_update = true;
        self.check_sink();
        let mut tran = Trans::None;
        self.input_cache.current_duration = self.get_progress();

        let mut loop_state = LoopState::wait_until(Duration::from_secs_f32(31.0 / 30.0), 0.001);

        if self.input_cache.current_duration >= self.total_duration {
            self.sink.pause();
        }

        if !self.sink.is_paused() {
            loop_state = LoopState::POLL;
        }

        if s.app
            .inputs
            .is_pressed(&[PhysicalKey::Code(KeyCode::Space)])
        {
            self.switch_play();
        }

        if s.app.inputs.is_pressed(&[
            PhysicalKey::Code(KeyCode::ControlLeft),
            PhysicalKey::Code(KeyCode::KeyS),
        ]) {
            self.save(s);
        }

        {
            if !s.app.egui_ctx.wants_keyboard_input() {
                let data = &mut self.input_cache.edit_data;
                if s.app
                    .inputs
                    .is_pressed(&[PhysicalKey::Code(KeyCode::Digit1)])
                {
                    data.cursor = PointerType::Select(None);
                }
                if s.app
                    .inputs
                    .is_pressed(&[PhysicalKey::Code(KeyCode::Digit2)])
                {
                    data.cursor = PointerType::NormalNote;
                }
                if s.app
                    .inputs
                    .is_pressed(&[PhysicalKey::Code(KeyCode::Digit3)])
                {
                    data.cursor = PointerType::LongNote(None);
                }
            }
        }

        match self.current_editor {
            SubEditor::Note => {
                self.update_note_editor(s);
            }
            _ => {}
        }

        let cur_input = &s.app.inputs.cur_frame_input;
        if cur_input
            .pressing
            .contains(&PhysicalKey::Code(KeyCode::Escape))
        {
            self.input_cache.escape_time += s.dt;
            if self.input_cache.escape_time >= 1.0 {
                tran = Trans::Pop;
            }
        } else {
            self.input_cache.escape_time = 0.0;
        }

        (tran, loop_state)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut tran = Trans::None;
        self.render_top_panel(s, ctx);
        if self.current_editor != SubEditor::Settings {
            self.render_top_audio_wave(s, ctx);
        }

        self.render_bottom_progress(s, ctx);

        match self.current_editor {
            SubEditor::Settings => {
                self.render_settings_editor(s, ctx);
            }
            SubEditor::Note => {
                self.render_note_editor(s, ctx);
            }
            SubEditor::Timing => {
                self.render_timing_editor(s, ctx);
            }
        }

        s.app.window.set_title(&format!(
            "{}{}",
            &self.beatmap.metadata.title,
            ["", " *"][self.dirty as usize]
        ));

        self.allow_update = false;
        tran
    }

    fn stop(&mut self, s: &mut StateData) {
        // Do save work
        self.save(s);
    }
}

impl BeatMapEditor {
    /// Get the position in game progress
    fn get_progress(&self) -> Duration {
        let dur = self.sink.get_pos();
        dur.div((1.0 / self.play_speed) as u32)
            .min(self.total_duration)
    }

    /// The pos in game pos
    fn seek_to(&self, pos: Duration) {
        if self.play_speed != 1.0 {
            self.sink
                .try_seek(pos.mul((1.0 / self.play_speed) as u32))
                .expect("Failed to seek");
        } else {
            self.sink.try_seek(pos).expect("Failed to seek");
        }
    }

    pub(crate) fn get_beat_iter(&self, secs: f32) -> TimingGroupBeatIterator {
        self.beatmap.timing_group.get_beat_iterator(
            self.input_cache.select_timing_group,
            (secs * 1000.0) as OffsetType,
            self.input_cache.detail,
        )
    }

    fn set_speed(&mut self, speed: f32) {
        let old_speed = self.play_speed;
        let playing = !self.sink.is_paused();
        if old_speed != speed {
            let pos = self.get_progress();
            self.sink.clear();
            self.play_speed = speed;
            self.check_sink();
            self.seek_to(pos);
            if playing {
                self.sink.play();
            }
        }

        // self.sink.set_speed(speed);

        // speed 1 -> 2
        // duration 30s -> 15s
        // self.sink
        //     .try_seek(old_dur.mul_f32(old_speed / speed))
        //     .expect("Failed to fix duration");
    }

    fn render_top_panel(&mut self, s: &mut StateData, ctx: &Context) {
        let height = 36.0;
        egui::TopBottomPanel::new(TopBottomSide::Top, "editor_top_menu")
            .frame(Frame::NONE)
            .min_height(height)
            .show(ctx, |ui| {
                let width = ui.available_width();

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let button_height = ui.available_height()
                        - ui.spacing().button_padding.y * 2.0
                        - ui.spacing().item_spacing.y * 2.0;
                    let button = Button::new("Timing")
                        .selected(self.current_editor == SubEditor::Timing)
                        .min_size(Vec2::new(0.0, button_height));

                    ui.add_space(ui.spacing().button_padding.x);
                    if ui.add(button).clicked() {
                        self.current_editor = SubEditor::Timing;
                    }
                    let button = Button::new("Beatmap")
                        .selected(self.current_editor == SubEditor::Note)
                        .min_size(Vec2::new(0.0, button_height));
                    if ui.add(button).clicked() {
                        self.current_editor = SubEditor::Note;
                    }

                    let button = Button::new("Settings")
                        .selected(self.current_editor == SubEditor::Settings)
                        .min_size(Vec2::new(0.0, button_height));
                    if ui.add(button).clicked() {
                        self.current_editor = SubEditor::Settings;
                    }
                });
            });
    }

    fn render_top_audio_wave(&mut self, s: &mut StateData, ctx: &Context) {
        let height = 100.0;
        egui::TopBottomPanel::new(TopBottomSide::Top, "editor_top_progress")
            .frame(Frame::NONE)
            .min_height(height + 25.0)
            .max_height(height + 25.0)
            .show(ctx, |ui| {
                let width = ui.available_width();
                let ui_height = ui.available_height();
                let right_width = 250.0;

                let start_point = ui.next_widget_position().add((0.0, 12.5).into());
                {
                    let start_point = ui.next_widget_position();
                    let ui_builder = UiBuilder::new().max_rect(Rect::from_min_max(
                        Pos2::new(start_point.x + width - right_width, start_point.y),
                        Pos2::new(start_point.x + width, start_point.y + ui_height),
                    ));
                    ui.allocate_new_ui(ui_builder, |ui| {
                        // [-, +]
                        let detail_dest = [
                            [1, 1],
                            // 1
                            [1, 2],
                            [1, 3],
                            [2, 4],
                            [3, 5],
                            [4, 6],
                            [5, 7],
                            [6, 8],
                            // 8
                            [7, 16],
                            [9, 9],
                            [10, 10],
                            [11, 11],
                            [12, 12],
                            [13, 13],
                            [14, 14],
                            [15, 15],
                            [8, 16],
                        ];

                        ui.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                            let minus_res = ui.button("-");
                            if minus_res.clicked() {
                                self.input_cache.detail =
                                    detail_dest[self.input_cache.detail as usize][0];
                            }

                            ui.label(format!("1 / {}", self.input_cache.detail));

                            let space =
                                (ui.available_width() - minus_res.rect.width()).at_least(0.0);
                            ui.add_space(space);
                            if ui.button("+").clicked() {
                                self.input_cache.detail =
                                    detail_dest[self.input_cache.detail as usize][1];
                            }
                        });
                    });
                }

                let progress_width = (width - right_width).ceil();
                if progress_width <= 1.0 {
                    return;
                }
                let background_rect = Rect {
                    min: start_point,
                    max: (start_point.x + progress_width, start_point.y + height).into(),
                };

                let wave_area_rect = Rect {
                    min: Pos2::new(start_point.x, start_point.y - 12.5),
                    max: (
                        start_point.x + progress_width,
                        start_point.y - 12.5 + ui_height,
                    )
                        .into(),
                };
                let raw_clip_rect = ui.clip_rect();
                // Clip to the wave area.
                ui.set_clip_rect(wave_area_rect);

                let response = ui.allocate_rect(background_rect, Sense::hover());
                if response.hover_pos().is_some() {
                    ui.input(|input| {
                        self.input_cache.progress_half_time -= input.smooth_scroll_delta.y * 0.01;
                        self.input_cache.progress_half_time =
                            self.input_cache.progress_half_time.clamp(0.2, 3.0);
                    });
                }
                ui.painter()
                    .rect_filled(background_rect, 0.0, Color32::DARK_GRAY);

                let now = self.input_cache.current_duration.as_secs_f32();
                let right_time = now + self.input_cache.progress_half_time;
                let left_time = now - (right_time - now);
                let time_len = right_time - left_time;

                let mut vec = Vec::new();

                // min, max
                vec.resize_with(progress_width as usize, || {
                    (AtomicI16::new(0), AtomicI16::new(0))
                });

                let time_to_wave_x =
                    |time: f32| (time - left_time) * progress_width / time_len + start_point.x;

                let raw_left_sample_idx =
                    (left_time * self.sample_info.sample_rate as f32) as isize;
                let left_sample_idx = raw_left_sample_idx.at_least(0) as usize;
                let right_sample_idx = (right_time * self.sample_info.sample_rate as f32) as usize;

                let idx_len = (right_sample_idx as isize - raw_left_sample_idx + 1) as usize;

                let right_sample_idx = right_sample_idx.at_most(
                    (self.total_duration.as_secs_f32() * self.sample_info.sample_rate as f32)
                        as usize,
                );

                use rayon::prelude::*;

                let left_pixel_start = raw_left_sample_idx * vec.len() as isize / idx_len as isize;

                (left_sample_idx.at_least(0)..=right_sample_idx)
                    .into_par_iter()
                    .for_each(|sample_idx| {
                        let (mut mn, mut mx) = (0, 0);
                        for j in 0..self.sample_info.channels as usize {
                            let cur = self.sample_info.samples
                                [sample_idx * self.sample_info.channels as usize + j];
                            mn = mn.min(cur);
                            mx = mx.max(cur);
                        }

                        let offset = sample_idx;

                        let pixel = offset * vec.len() / idx_len;
                        let pixel = (pixel as isize - left_pixel_start) as usize;

                        if let Some((x, y)) = vec.get(pixel) {
                            x.fetch_min(mn, Ordering::Relaxed);
                            y.fetch_max(mx, Ordering::Relaxed);
                        }
                    });

                let painter = ui.painter();

                let mx_val = (i16::MIN as f32).abs();

                let center_y = start_point.y + height * 0.5;
                let half_height = height * 0.5;

                // Render the wave.
                if true {
                    vec.iter_mut().enumerate().for_each(|(offset, (mn, mx))| {
                        let high = *mx.get_mut() as f32 / mx_val;
                        let low = *mn.get_mut() as f32 / mx_val;

                        let high = center_y - high.abs() * half_height;
                        let low = center_y + low.abs() * half_height;
                        let color = Color32::from_rgb(108, 172, 200);
                        painter.vline(
                            start_point.x + offset as f32,
                            high..=low,
                            Stroke::new(1.125, color),
                        );
                    });
                }

                // render timings lines

                self.beatmap
                    .timing_group
                    .get_beat_iterator(
                        self.input_cache.select_timing_group,
                        secs_to_offset_type(left_time),
                        self.input_cache.detail,
                    )
                    .filter(|x| x.number >= 0)
                    .try_for_each(|beat| {
                        let beat_x = time_to_wave_x(beat.time as f32 / 1000.0);
                        if beat_x > start_point.x + progress_width + 5.0 {
                            return ControlFlow::Break(());
                        }
                        let width = if beat.is_measure { 3.0 } else { 2.0 };
                        let range = if beat.is_measure {
                            start_point.y - 6.25..=start_point.y + height + 6.25
                        } else {
                            start_point.y..=start_point.y + height
                        };
                        let color = beat.get_color();

                        ui.painter().vline(beat_x, range, Stroke::new(width, color));

                        ControlFlow::Continue(())
                    });

                ui.set_clip_rect(raw_clip_rect);

                // render current line
                ui.painter().vline(
                    start_point.x + progress_width * 0.5,
                    start_point.y - 12.5..=start_point.y - 12.5 + ui_height,
                    Stroke::new(5.0, Color32::LIGHT_BLUE),
                );
            });
    }

    fn render_bottom_progress(&mut self, s: &mut StateData, ctx: &Context) {
        let height = 50.0;
        egui::TopBottomPanel::new(TopBottomSide::Bottom, "audio")
            .min_height(height)
            .frame(Frame::none())
            .show(ctx, |ui| {
                let width = ui.available_width();

                let start_point = ui.next_widget_position();

                let progress_width = width - 200.0 - 100.0 * 4.0;

                let cur_progress = self.get_progress();
                let left_height = height - 25.0;

                ui.allocate_ui(Vec2::new(200.0, height), |ui| {
                    ui.allocate_ui(Vec2::new(200.0, left_height), |ui| {
                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            let mut progress_str = format_duration(&cur_progress);
                            const ID: &'static str = "PROGRESS";

                            let mut cache = get_edit_cache();

                            let the_str = if cache.is_editing(ID) {
                                &mut cache.text
                            } else {
                                &mut progress_str
                            };

                            ui.add_space(ui.style().spacing.item_spacing.y);

                            let text_edit = TextEdit::singleline(the_str)
                                .font(TextStyle::Heading)
                                .horizontal_align(Align::Center);

                            let response = ui.add(text_edit);

                            if response.has_focus() {
                                cache.edit(&progress_str, ID);
                            } else if response.lost_focus() {
                                if let Some(dur) = get_duration_from_str(&cache.text) {
                                    self.seek_to(dur);
                                }
                                cache.release();
                            }
                            ui.add_space((left_height - ui.min_rect().height()).at_least(0.0));
                        });
                    });
                });

                ui.allocate_new_ui(
                    UiBuilder::new().max_rect(Rect {
                        min: (start_point.x + 200.0, start_point.y).into(),
                        max: (start_point.x + width - 400.0, start_point.y + height).into(),
                    }),
                    |ui| {
                        let start_point = ui.next_widget_position();
                        let padding = 5.0;

                        let y_center = start_point.y + height * 0.5;

                        let progress_width = progress_width - padding * 2.0;

                        let progress_start = ui.next_widget_position().x + 5.0;

                        let progress_end = progress_start + progress_width;

                        let painter = ui.painter();
                        let y_range = start_point.y..=start_point.y + height;

                        let cur_progress = (self.get_progress().as_secs_f64()
                            / self.total_duration.as_secs_f64())
                            as f32;

                        let left_center = (progress_start, y_center);
                        let right_center = (progress_end, y_center);

                        for x in self.beatmap.timing_group.get_timing(self.input_cache.select_timing_group, 0) {
                            let progress = offset_type_to_secs(x.offset) / self.total_duration.as_secs_f64();
                            let progress = progress as f32;

                            if x.set_bpm.is_some() {
                                painter.vline(
                                    progress_start + progress_width * progress,
                                    y_range.clone(),
                                    Stroke::new(1.0, Color32::BLUE),
                                );
                            } else if x.set_speed.is_some() {
                                painter.vline(
                                    progress_start + progress_width * progress,
                                    y_range.clone(),
                                    Stroke::new(1.0, Color32::GREEN),
                                );
                            }
                        }
                        painter.vline(
                            progress_start + progress_width * cur_progress,
                            y_range.clone(),
                            Stroke::new(1.0, Color32::RED),
                        );
                        painter.hline(
                            progress_start..=progress_end,
                            start_point.y + height * 0.5,
                            Stroke::new(1.0, Color32::WHITE),
                        );

                        painter.circle_filled(left_center.into(), 3.0, Color32::WHITE);
                        painter.circle_filled(right_center.into(), 3.0, Color32::WHITE);

                        let progress_rect = Rect {
                            min: (progress_start, start_point.y).into(),
                            max: (progress_end, start_point.y + height).into(),
                        };

                        let response = ui.allocate_rect(progress_rect, Sense::drag());
                        if (response.dragged() && response.drag_delta().length_sq() != 0.0)
                            || response.drag_started()
                        {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let drag_x = pos.x;
                                let drag_progress =
                                    ((drag_x - progress_start) / progress_width).clamp(0.0, 1.0);

                                let dest_duration = self.total_duration.mul_f32(drag_progress);

                                self.seek_to(dest_duration);
                            }
                        } else if response.contains_pointer() {
                            self.scroll_beat(ui);
                        }
                    },
                );

                let button_height = height - 10.0;

                ui.allocate_new_ui(
                    UiBuilder::new().max_rect(Rect {
                        min: (start_point.x + width - 400.0, start_point.y + 5.0).into(),
                        max: (start_point.x + width, start_point.y + 5.0 + button_height).into(),
                    }),
                    |ui| {
                        ui.horizontal(|ui| {
                            let text = if self.sink.is_paused() {
                                "Play"
                            } else {
                                "Pause"
                            };
                            let item_size = (400.0 - ui.style().spacing.item_spacing.x * 4.0) / 5.0;
                            let cell_size = (item_size, button_height);

                            let play_button = Button::new(text).min_size(cell_size.into());

                            if ui.add_sized(cell_size, play_button).clicked() {
                                self.switch_play();
                            }

                            for speed in [0.25, 0.5, 0.75, 1.0] {
                                if ui
                                    .add_sized(
                                        cell_size,
                                        Button::new(speed.to_string()).min_size(cell_size.into()),
                                    )
                                    .clicked()
                                {
                                    self.set_speed(speed);
                                }
                            }
                        })
                    },
                )
            });
    }

    fn check_sink(&self) {
        let start_time = Instant::now();
        if self.sink.empty() {
            let speed = self.play_speed;

            match speed {
                0.25 => {
                    let samples = SamplesBuffer::new(
                        self.sample_info.channels,
                        self.sample_info.sample_rate,
                        self.sample_info.samples_q.clone(),
                    );

                    self.sink.append(samples);
                }
                0.5 => {
                    let samples = SamplesBuffer::new(
                        self.sample_info.channels,
                        self.sample_info.sample_rate,
                        self.sample_info.samples_half.clone(),
                    );
                    self.sink.append(samples);
                }
                0.75 => {
                    let samples = SamplesBuffer::new(
                        self.sample_info.channels,
                        self.sample_info.sample_rate,
                        self.sample_info.samples_t_f.clone(),
                    );

                    self.sink.append(samples);
                }
                _ => {
                    let samples = SamplesBuffer::new(
                        self.sample_info.channels,
                        self.sample_info.sample_rate,
                        self.sample_info.samples.clone(),
                    );
                    log::info!("Reuse raw samples");

                    self.sink.append(samples);
                }
            }

            log::info!(
                "Checked sink and re append buffer in {}s with speed {}",
                start_time.elapsed().as_secs_f64(),
                self.play_speed
            );
        }
    }

    fn switch_play(&self) {
        if self.sink.is_paused() {
            if self.get_progress() + Duration::from_millis(1) >= self.total_duration {
                self.sink
                    .try_seek(Duration::new(0, 0))
                    .expect("Seek failed");
            }

            self.sink.play();
        } else {
            self.sink.pause();
        }
    }

    pub fn scroll_beat(&self, ui: &mut Ui) {
        ui.input(|input| {
            if input.raw_scroll_delta.y == 0.0 {
                return;
            }
            let (left, _, right) = self.beatmap.timing_group.get_near_beat(
                self.input_cache.select_timing_group,
                self.input_cache.current_duration.as_millis() as OffsetType,
                self.input_cache.detail,
            );
            let dest_time = if input.raw_scroll_delta.y < 0.0 {
                // go right
                right.time
            } else {
                // go left
                left.time
            }
            .clamp(0, self.total_duration.as_millis() as OffsetType);

            self.seek_to(Duration::from_millis(dest_time as u64));
        });
    }
}

pub fn format_duration(dur: &Duration) -> String {
    let ms = dur.as_millis();
    format_ms(ms as i128)
}

pub fn format_ms(ms: i128) -> String {
    let s = ms / 1000;
    let min = s / 60;

    format!("{:02}:{:02}:{:03}", min, s % 60, ms % 1000)
}

pub fn get_duration_from_str(str: &str) -> Option<Duration> {
    let mut it = str.rsplitn(3, ":");
    let ms = it.next()?.parse::<u64>().ok()?;
    let s = it.next().map(|x| x.parse::<u64>().ok());
    let m = it.next().map(|x| x.parse::<u64>().ok());

    if let Some(m) = m {
        let m = m?;
        let s = s.unwrap()?;
        Some(Duration::from_millis(
            ms.checked_add(
                s.checked_mul(1000)?
                    .checked_add(m.checked_mul(1000 * 60)?)?,
            )?,
        ))
    } else if let Some(s) = s {
        let m = s?;
        let s = ms;
        let ms = 0u64;
        Some(Duration::from_millis(
            ms.checked_add(
                s.checked_mul(1000)?
                    .checked_add(m.checked_mul(1000 * 60)?)?,
            )?,
        ))
    } else {
        let m = 0u64;
        let s = 0u64;
        Some(Duration::from_millis(
            ms.checked_add(
                s.checked_mul(1000)?
                    .checked_add(m.checked_mul(1000 * 60)?)?,
            )?,
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::state::editor::editor::get_duration_from_str;
    use std::time::Duration;

    #[test]
    fn test_parse_dur() {
        assert_eq!(get_duration_from_str("1"), Some(Duration::from_millis(1)));
        assert_eq!(get_duration_from_str("0"), Some(Duration::from_millis(0)));
        assert_eq!(
            get_duration_from_str("1:2:3"),
            Some(Duration::from_millis(3 + 2 * 1000 + 1 * 1000 * 60))
        );
        assert_eq!(
            get_duration_from_str("1234"),
            Some(Duration::from_millis(1234))
        );
        assert_eq!(
            get_duration_from_str("1:0"),
            Some(Duration::from_millis(60 * 1000))
        );
        assert_eq!(
            get_duration_from_str("1:50"),
            Some(Duration::from_millis(60 * 1000 + 50 * 1000))
        );
    }
}
