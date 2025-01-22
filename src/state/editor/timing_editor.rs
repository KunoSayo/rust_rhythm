use crate::engine::StateData;
use crate::game::timing::Timing;
use crate::game::OffsetType;
use crate::state::editor::editor::{format_ms, BeatMapEditor};
use egui::panel::Side;
use egui::{Button, Frame, NumExt};
use egui_extras::Column;

impl BeatMapEditor {
    pub fn render_timing_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
        let mut op: std::cell::Cell<Option<Box<dyn FnOnce(&mut Self)>>> = Default::default();
        let last_selected_group = self.input_cache.select_timing_group;
        let last_selected_row = self.input_cache.select_timing_row;
        let now = self.input_cache.current_duration.as_millis() as OffsetType;

        egui::SidePanel::new(Side::Left, "timing_left")
            .frame(Frame::none())
            .max_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::new([false, true])
                    .show(ui, |ui| {
                        ui.set_max_width(200.0);
                        ui.vertical_centered(|ui| {
                            let current_selected = self.input_cache.select_timing_group;
                            for (idx, _) in self.song_beatmap_file.timing_group.timing_lines.iter().enumerate() {
                                // We battle with borrow checker....
                                ui.set_max_width(200.0);
                                egui::Sides::new()
                                    .spacing(20.0)
                                    .show(ui, |ui| {
                                        ui.set_max_width(80.0);
                                        ui.centered_and_justified(|ui| {
                                            let idx = idx;
                                            if ui.add(Button::new(idx.to_string()).selected(current_selected == idx)).clicked() {
                                                op.set(Some(Box::new(move |this: &mut Self| this.input_cache.select_timing_group = idx)));
                                            }
                                        });
                                    }, |ui| {
                                        if idx == 0 {
                                            return;
                                        }
                                        ui.set_max_width(90.0);
                                        ui.add_space(10.0);
                                        ui.centered_and_justified(|ui| {
                                            if ui.add(Button::new("X")).clicked() {
                                                op.set(Some(Box::new(move |this: &mut Self| {
                                                    // todo: Delete the note after deleting the timing group. (really?)
                                                    if this.input_cache.select_timing_group >= idx {
                                                        this.input_cache.select_timing_group = this.input_cache.select_timing_group
                                                            .saturating_sub(1)
                                                    }
                                                    this.song_beatmap_file.timing_group.timing_lines.remove(idx);
                                                })));
                                            }
                                        });
                                    });
                            }


                            ui.centered_and_justified(|ui| {
                                if ui.add(Button::new("+")).clicked() {
                                    self.song_beatmap_file.timing_group.timing_lines.push(Default::default());
                                }
                            })
                        })
                    });
            });
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let text_height = egui::TextStyle::Body
                    .resolve(ui.style())
                    .size
                    .max(ui.spacing().interact_size.y);

                let ui_height = ui.available_height();
                let table_height = ui_height - 150.0;
                let table = egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .max_scroll_height(table_height)
                    .column(Column::auto().at_least(200.0))
                    .column(Column::remainder());

                let table = table.sense(egui::Sense::click());


                let result = table.header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("TIME");
                    });
                    header.col(|ui| {
                        ui.heading("ATTRIBUTES");
                    });
                }).body(|mut body| {
                    if let Some(tl) = self.song_beatmap_file.timing_group.timing_lines.get(self.input_cache.select_timing_group) {
                        body.rows(text_height, tl.timings.len(), |mut row| {
                            let idx = row.index();
                            let timing = &tl.timings[idx];
                            row.set_selected(self.input_cache.select_timing_row.map(|x| x == idx).unwrap_or(false));
                            row.col(|ui| {
                                ui.label(format_ms(timing.offset as i128));
                            });
                            row.col(|ui| {
                                ui.label(format!("BPM: {}", timing.bpm));
                            });

                            if row.response().clicked() {
                                self.input_cache.select_timing_row = Some(idx);
                            }
                        })
                    } else {
                        body.rows(text_height, 0, |_| {});
                    }
                });

                let the_space = (table_height - result.content_size.y).at_least(0.0);
                ui.add_space(the_space);

                let same_time_with_timing = self.song_beatmap_file.timing_group.has_timing(last_selected_group, now);
                egui::Sides::new()
                    .show(ui, |ui| {}, |ui| {
                        if ui.add_enabled(!same_time_with_timing, Button::new("Add").min_size([200.0, 100.0].into())).clicked() {
                            op.set(Some(Box::new(|this| {
                                if let Some(group) = this.song_beatmap_file.timing_group.timing_lines.get_mut(this.input_cache.select_timing_group) {
                                    group.add_new(Timing::create_from_offset(this.input_cache.current_duration.as_millis() as OffsetType))
                                }
                            })));
                        }
                    });
            });

        if let Some(op) = op.get_mut().take() {
            op(self)
        }
    }
}