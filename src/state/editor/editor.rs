use crate::engine::global::{IO_POOL, STATIC_DATA};
use crate::engine::{GameState, LoopState, StateData, Trans};
use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::{SongBeatmapInfo, BEATMAP_EXT};
use crate::game::song::{SongInfo, SongManagerResourceType};
use crate::game::secs_to_offset_type;
use anyhow::anyhow;
use egui::epaint::PathStroke;
use egui::panel::TopBottomSide;
use egui::{Align, Button, Color32, Context, Frame, Label, Layout, NumExt, Pos2, Rect, RichText, Sense, UiBuilder, Vec2};
use rodio::{Decoder, OutputStreamHandle, Sink, Source};
use std::io::{Cursor, Read};
use std::ops::{Add, ControlFlow, Deref};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct SongSampleInfo {
    raw_data: Cursor<Vec<u8>>,
    samples: Vec<i16>,
    sample_rate: u32,
    channels: u16,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SubEditor {
    Settings,
    Beatmap,
    Timing,
}
pub struct BeatMapEditor {
    pub song_info: Arc<SongInfo>,
    pub song_beatmap_file: SongBeatmapFile,
    save_path: Option<PathBuf>,
    total_duration: Duration,
    sink: Sink,
    pub(in crate::state::editor) input_cache: InputCache,

    sample_info: SongSampleInfo,

    current_editor: SubEditor,
}


pub(in crate::state::editor) struct InputCache {
    pub(in crate::state::editor) escape_time: f32,
    pub(in crate::state::editor) current_duration: Duration,
    pub(in crate::state::editor) progress_half_time: f32,
    pub(in crate::state::editor) select_timing_group: usize,
}

impl Default for InputCache {
    fn default() -> Self {
        Self {
            escape_time: 0.0,
            current_duration: Default::default(),
            progress_half_time: 1.0,
            select_timing_group: 0,
        }
    }
}

impl BeatMapEditor {
    pub fn new(song_info: Arc<SongInfo>, handle: OutputStreamHandle) -> anyhow::Result<Self> {
        Self::with_file(song_info, None, handle)
    }

    pub fn with_file(song_info: Arc<SongInfo>, info: Option<SongBeatmapInfo>, s: OutputStreamHandle) -> anyhow::Result<Self> {
        let sink = Sink::try_new(&s)
            .expect("Failed to new sink");

        let mut buf = vec![];
        let mut file = std::fs::File::open(&song_info.bgm_file)?;
        file.read_to_end(&mut buf)?;

        let buf = Cursor::new(buf);
        let decoder = Decoder::new(buf.clone())?;

        let samples = decoder.convert_samples::<f32>();

        let total_duration = samples.total_duration().ok_or(anyhow!("No audio duration"))?;
        sink.pause();
        sink.append(samples);


        let vol = STATIC_DATA.cfg_data.write()
            .map_err(|e| anyhow!("Cannot read lock for {:?}", e))?
            .get_f32_def("bgm_vol", 1.0);
        sink.set_volume(vol);

        let path = info.as_ref().map(|x| x.file_path.clone());

        let sample_info = {
            let decoder = Decoder::new(buf.clone())?;

            let sample_rate = decoder.sample_rate();
            let channels = decoder.channels();
            let samples = decoder.convert_samples::<i16>().collect();
            SongSampleInfo {
                raw_data: buf,
                samples,
                sample_rate,
                channels,
            }
        };

        let current_editor = SubEditor::Timing;
        Ok(Self {
            song_beatmap_file: info.map(|x| x.song_beatmap_file).unwrap_or(SongBeatmapFile::new(song_info.title.clone())),
            song_info,
            sink,
            save_path: path,
            total_duration,
            input_cache: Default::default(),
            sample_info,
            current_editor,
        })
    }
}


impl GameState for BeatMapEditor {
    fn start(&mut self, s: &mut StateData) {}

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        self.check_sink();
        let mut tran = Trans::None;
        self.input_cache.current_duration = self.get_progress();

        let mut loop_state = LoopState::WAIT;

        if self.input_cache.current_duration >= self.total_duration {
            self.sink.pause();
        }

        if !self.sink.is_paused() {
            loop_state = LoopState::POLL;
        }

        if s.app.inputs.is_pressed(&[PhysicalKey::Code(KeyCode::Space)]) {
            self.switch_play();
        }

        let cur_input = &s.app.inputs.cur_frame_input;
        if cur_input.pressing.contains(&PhysicalKey::Code(KeyCode::Escape)) {
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
            SubEditor::Beatmap => {}
            SubEditor::Timing => {
                self.render_timing_editor(s, ctx);
            }
        }

        tran
    }

    fn stop(&mut self, s: &mut StateData) {

        // Do save work
        if self.save_path.is_none() &&
            (self.song_beatmap_file.metadata.title.is_empty() || self.song_beatmap_file.metadata.version.is_empty()) {
            return;
        }
        let path = self.save_path.get_or_insert_with(|| {
            self.song_info.bgm_file.parent().unwrap()
                .join(format!("{}[{}]", &self.song_beatmap_file.metadata.title, &self.song_beatmap_file.metadata.version)
                    + "." + BEATMAP_EXT)
        });
        let path = path.clone();
        let beatmap = self.song_beatmap_file.clone();
        let info = self.song_info.clone();
        let song_manager = s.wd.world.fetch::<SongManagerResourceType>().deref().clone();
        IO_POOL.spawn_ok(async move {
            if let Err(e) = beatmap.save_to(&path) {
                log::error!("Failed to save beatmap for {:?}", e);
            } else {
                match info.reload() {
                    Ok(new_info) => {
                        song_manager.load_new_info(new_info);
                        info.dirty.store(true, Ordering::Release);
                    }
                    Err(e) => {
                        log::error!("Failed to load beatmap for {:?}", e);
                    }
                }
            }
        });
    }
}

impl BeatMapEditor {
    fn get_progress(&self) -> Duration {
        let dur = self.sink.get_pos();
        dur.mul_f32(self.sink.speed()).min(self.total_duration)
    }

    fn set_speed(&self, speed: f32) {
        let old_dur = self.sink.get_pos();
        let old_speed = self.sink.speed();
        self.sink.set_speed(speed);

        // speed 1 -> 2
        // duration 30s -> 15s
        self.sink.try_seek(old_dur.mul_f32(old_speed / speed))
            .expect("Failed to fix duration");
    }

    fn render_top_panel(&mut self, s: &mut StateData, ctx: &Context) {
        let height = 100.0;
        egui::TopBottomPanel::new(TopBottomSide::Top, "editor_top_menu")
            .frame(Frame::none())
            .min_height(height)
            .show(ctx, |ui| {
                let width = ui.available_width();

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let button_height = ui.available_height() - ui.spacing().button_padding.y * 2.0 - ui.spacing().item_spacing.y * 2.0;
                    let button = Button::new("Timing").selected(self.current_editor == SubEditor::Timing).min_size(Vec2::new(0.0, button_height));

                    ui.add_space(ui.spacing().button_padding.x);
                    if ui.add(button).clicked() {
                        self.current_editor = SubEditor::Timing;
                    }
                    let button = Button::new("Beatmap").selected(self.current_editor == SubEditor::Beatmap).min_size(Vec2::new(0.0, button_height));
                    if ui.add(button).clicked() {
                        self.current_editor = SubEditor::Beatmap;
                    }

                    let button = Button::new("Settings").selected(self.current_editor == SubEditor::Settings).min_size(Vec2::new(0.0, button_height));
                    if ui.add(button).clicked() {
                        self.current_editor = SubEditor::Settings;
                    }
                });
            });
    }


    fn render_top_audio_wave(&mut self, s: &mut StateData, ctx: &Context) {
        let height = 100.0;
        egui::TopBottomPanel::new(TopBottomSide::Top, "editor_top_progress")
            .frame(Frame::none())
            .min_height(height + 25.0)
            .show(ctx, |ui| {
                let width = ui.available_width();
                let ui_height = ui.available_height();
                let right_width = 250.0;
                let progress_width = (width - right_width).ceil();
                let start_point = ui.next_widget_position().add((0.0, 12.5).into());
                let background_rect = Rect {
                    min: start_point,
                    max: (start_point.x + progress_width, start_point.y + height).into(),
                };

                let wave_area_rect = Rect {
                    min: Pos2::new(start_point.x, start_point.y - 12.5),
                    max: (start_point.x + progress_width, start_point.y - 12.5 + ui_height).into(),
                };
                let raw_clip_rect = ui.clip_rect();
                // Clip to the wave area.
                ui.set_clip_rect(wave_area_rect);

                let response = ui.allocate_rect(background_rect, Sense::hover());
                if response.hover_pos().is_some() {
                    ui.input(|input| {
                        self.input_cache.progress_half_time -= input.smooth_scroll_delta.y * 0.01;
                        self.input_cache.progress_half_time = self.input_cache.progress_half_time.clamp(0.2, 3.0);
                    });
                }
                ui.painter().rect_filled(background_rect, 0.0, Color32::DARK_GRAY);


                let now = self.input_cache.current_duration.as_secs_f32();
                let right_time = now + self.input_cache.progress_half_time;
                let left_time = now - (right_time - now);
                let time_len = right_time - left_time;

                let mut vec = Vec::new();

                // min, max
                vec.resize_with(progress_width as usize, || (AtomicI16::new(0), AtomicI16::new(0)));

                let time_to_wave_x = |time: f32| {
                    (time - left_time) * progress_width / time_len + start_point.x
                };

                let raw_left_sample_idx = (left_time * self.sample_info.sample_rate as f32) as isize;
                let left_sample_idx = raw_left_sample_idx.at_least(0) as usize;
                let right_sample_idx = (right_time * self.sample_info.sample_rate as f32) as usize;

                let idx_len = (right_sample_idx as isize - raw_left_sample_idx + 1) as usize;

                let right_sample_idx = right_sample_idx.at_most((self.total_duration.as_secs_f32() * self.sample_info.sample_rate as f32) as usize);

                use rayon::prelude::*;

                let left_pixel_start = raw_left_sample_idx * vec.len() as isize / idx_len as isize;

                (left_sample_idx.at_least(0)..=right_sample_idx).into_par_iter().for_each(|sample_idx| {
                    let (mut mn, mut mx) = (0, 0);
                    for j in 0..self.sample_info.channels as usize {
                        let cur = self.sample_info.samples[sample_idx * self.sample_info.channels as usize + j];
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

                vec.par_iter_mut().enumerate().for_each(|(offset, (mn, mx))| {
                    let high = *mx.get_mut() as f32 / mx_val;
                    let low = *mn.get_mut() as f32 / mx_val;

                    let high = center_y - high.abs() * half_height;
                    let low = center_y + low.abs() * half_height;
                    let color = Color32::from_rgb(108, 172, 200);
                    painter.vline(start_point.x + offset as f32, high..=low, PathStroke::new(1.125, color));
                });

                // render timings lines

                self.song_beatmap_file.timing_group
                    .get_beat_iterator(self.input_cache.select_timing_group, secs_to_offset_type(left_time))
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
                        let color = Color32::from_gray(if beat.is_measure { 233 } else { 222 });
                        ui.painter().vline(beat_x, range, PathStroke::new(width, color));

                        ControlFlow::Continue(())
                    });

                ui.set_clip_rect(raw_clip_rect);


                // render current line
                ui.painter().vline(start_point.x + progress_width * 0.5, start_point.y - 12.5..=start_point.y - 12.5 + ui_height,
                                   PathStroke::new(5.0, Color32::LIGHT_BLUE));
            });
    }

    fn render_bottom_progress(&mut self, s: &mut StateData, ctx: &Context) {
        let height = 100.0;
        egui::TopBottomPanel::new(TopBottomSide::Bottom, "audio")
            .min_height(height)
            .show(ctx, |ui| {
                let width = ui.available_width();

                let start_point = ui.next_widget_position();

                let progress_width = width - 200.0 - 100.0 * 4.0;

                let cur_progress = self.get_progress();

                ui.allocate_ui(Vec2::new(200.0, height), |ui| {
                    let progress_str = format_duration(&cur_progress);
                    let text = RichText::new(progress_str)
                        .size(35.0);
                    let label = Label::new(text)
                        .selectable(true);

                    ui.allocate_ui(Vec2::new(200.0, 75.0), |ui| {
                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            ui.add_sized(ui.available_size(), label);
                        })
                    });
                });

                ui.allocate_new_ui(UiBuilder::new()
                                       .max_rect(Rect {
                                           min: (start_point.x + 200.0, start_point.y).into(),
                                           max: (start_point.x + width - 400.0, start_point.y + height).into(),
                                       }), |ui| {
                    let start_point = ui.next_widget_position();
                    let padding = 5.0;

                    let y_center = start_point.y + height * 0.5;

                    let progress_width = progress_width - padding * 2.0;

                    let progress_start = ui.next_widget_position().x + 5.0;

                    let progress_end = progress_start + progress_width;

                    let painter = ui.painter();
                    let y_range = start_point.y..=start_point.y + height;

                    let cur_progress = (self.get_progress().as_secs_f64() / self.total_duration.as_secs_f64()) as f32;

                    let left_center = (progress_start, y_center);
                    let right_center = (progress_end, y_center);
                    painter.vline(progress_start + progress_width * cur_progress, y_range, PathStroke::new(1.0, Color32::RED));
                    painter.hline(progress_start..=progress_end, start_point.y + height * 0.5, PathStroke::new(1.0, Color32::WHITE));

                    painter.circle_filled(left_center.into(), 3.0, Color32::WHITE);
                    painter.circle_filled(right_center.into(), 3.0, Color32::WHITE);

                    let progress_rect = Rect {
                        min: (progress_start, start_point.y).into(),
                        max: (progress_end, start_point.y + height).into(),
                    };

                    let response = ui.allocate_rect(progress_rect, Sense::drag());
                    if (response.dragged() && response.drag_delta().length_sq() != 0.0) || response.drag_started() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let drag_x = pos.x;
                            let drag_progress = ((drag_x - progress_start) / progress_width).clamp(0.0, 1.0);


                            let dest_duration = self.total_duration.mul_f32(drag_progress);


                            self.sink.try_seek(dest_duration.mul_f32(1.0 / self.sink.speed()))
                                .expect("Failed to seek");
                        }
                    }
                });


                ui.allocate_new_ui(UiBuilder::new()
                                       .max_rect(Rect {
                                           min: (start_point.x + width - 400.0, start_point.y + 25.0).into(),
                                           max: (start_point.x + width, start_point.y + 75.0).into(),
                                       }), |ui| {
                    ui.horizontal(|ui| {
                        let text = if self.sink.is_paused() {
                            "Play"
                        } else {
                            "Pause"
                        };
                        let item_size = (400.0 - ui.style().spacing.item_spacing.x * 4.0) / 5.0;
                        let cell_size = (item_size, 50.0);

                        let play_button = egui::Button::new(text)
                            .min_size(cell_size.into());

                        if ui.add_sized(cell_size, play_button).clicked() {
                            self.switch_play();
                        }


                        for speed in [0.25, 0.5, 0.75, 1.0] {
                            if ui.add_sized(cell_size, egui::Button::new(speed.to_string()).min_size(cell_size.into())).clicked() {
                                self.set_speed(speed);
                            }
                        }
                    })
                })
            });
    }

    fn check_sink(&self) {
        if self.sink.empty() {
            let decoder = Decoder::new(self.sample_info.raw_data.clone())
                .expect("We should not failed");

            let samples = decoder.convert_samples::<f32>();

            self.sink.append(samples);
        }
    }

    fn switch_play(&self) {
        if self.sink.is_paused() {
            if self.get_progress() + Duration::from_millis(1) >= self.total_duration {
                self.sink.try_seek(Duration::new(0, 0)).expect("Seek failed");
            }

            self.sink.play();
        } else {
            self.sink.pause();
        }
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

