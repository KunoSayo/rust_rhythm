use crate::engine::{optional_edit, StateData};
use crate::game::timing::{Bpm, Timing};
use crate::game::OffsetType;
use crate::state::editor::editor::{format_ms, BeatMapEditor};
use egui::panel::Side;
use egui::{Button, Frame, NumExt, Sense};
use egui_extras::Column;

impl BeatMapEditor {
    pub fn render_timing_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
        let mut op: std::cell::Cell<Option<Box<dyn FnOnce(&mut Self)>>> = Default::default();
        let last_selected_group = self.input_cache.select_timing_group;
        let last_selected_row = self.input_cache.select_timing_row;
        let now = self.input_cache.current_duration.as_millis() as OffsetType;

        egui::SidePanel::new(Side::Left, "timing_left")
            .frame(Frame::NONE)
            .max_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.set_max_width(200.0);
                    ui.vertical_centered(|ui| {
                        let current_selected = self.input_cache.select_timing_group;
                        for (idx, _) in self.beatmap.timing_group.timing_lines.iter().enumerate() {
                            // We battle with borrow checker....
                            ui.set_max_width(200.0);
                            egui::Sides::new().spacing(20.0).show(
                                ui,
                                |ui| {
                                    ui.set_max_width(80.0);
                                    ui.centered_and_justified(|ui| {
                                        let idx = idx;
                                        if ui
                                            .add(
                                                Button::new(idx.to_string())
                                                    .selected(current_selected == idx),
                                            )
                                            .clicked()
                                        {
                                            op.set(Some(Box::new(move |this: &mut Self| {
                                                this.input_cache.select_timing_group = idx
                                            })));
                                        }
                                    });
                                },
                                |ui| {
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
                                                    this.input_cache.select_timing_group = this
                                                        .input_cache
                                                        .select_timing_group
                                                        .saturating_sub(1)
                                                }
                                                this.beatmap.timing_group.timing_lines.remove(idx);
                                            })));
                                        }
                                    });
                                },
                            );
                        }

                        ui.centered_and_justified(|ui| {
                            if ui.add(Button::new("+")).clicked() {
                                self.beatmap
                                    .timing_group
                                    .timing_lines
                                    .push(Default::default());
                            }
                        })
                    })
                });
            });

        egui::SidePanel::right("timing_editor_right")
            .frame(Frame::NONE)
            .resizable(false)
            .max_width(200.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    if let Some(selected_row) = last_selected_row {
                        if let Some(tl) = self
                            .beatmap
                            .timing_group
                            .get_timing_by_idx(self.input_cache.select_timing_group, selected_row)
                        {
                            ui.label("BPM: ");

                            const ID: &'static str = "BPM_EDIT";

                            self.dirty |= optional_edit(ui, ID, "Set BPM", &mut tl.set_bpm, Bpm::default());

                            ui.separator();
                            const SET_SPEED: &'static str = "Set Speed";
                            self.dirty |= optional_edit(ui, SET_SPEED, SET_SPEED, &mut tl.set_speed, 1.0);
                            tl.set_speed = tl.set_speed.map(|x| x.clamp(0.01, 10.0));
                        }
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(Frame::NONE)
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

                let table = table.sense(Sense::click());

                let result = table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.heading("TIME");
                        });
                        header.col(|ui| {
                            ui.heading("ATTRIBUTES");
                        });
                        if header.response().clicked() {
                            self.input_cache.select_timing_row = None;
                        }
                    })
                    .body(|body| {
                        if let Some(tl) = self
                            .beatmap
                            .timing_group
                            .timing_lines
                            .get(self.input_cache.select_timing_group)
                        {
                            body.rows(text_height, tl.timings.len(), |mut row| {
                                let idx = row.index();
                                let timing = &tl.timings[idx];
                                row.set_selected(
                                    self.input_cache
                                        .select_timing_row
                                        .map(|x| x == idx)
                                        .unwrap_or(false),
                                );
                                row.col(|ui| {
                                    ui.label(format_ms(timing.offset as i128));
                                });
                                row.col(|ui| match (timing.set_bpm, timing.set_speed) {
                                    (Some(bpm), None) => {
                                        ui.label(format!("BPM: {}", bpm));
                                    }
                                    (None, Some(speed)) => {
                                        ui.label(format!("Speed: {}", speed));
                                    }
                                    (Some(bpm), Some(speed)) => {
                                        ui.label(format!("BPM: {}, Speed: {}", bpm, speed));
                                    }
                                    _ => {
                                        ui.label("Empty");
                                    }
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
                let (id, rect) = ui.allocate_space([ui.available_width(), the_space].into());
                if ui.interact(rect, id, Sense::click()).clicked() {
                    self.input_cache.select_timing_row = None;
                }

                let same_time_with_timing = self
                    .beatmap
                    .timing_group
                    .has_timing(last_selected_group, now);
                egui::Sides::new().show(
                    ui,
                    |ui| {
                        let result = last_selected_row
                            .map(|x| {
                                self.beatmap
                                    .timing_group
                                    .get_timing_by_idx(last_selected_group, x)
                            })
                            .flatten();

                        match result {
                            Some(_) => {
                                if ui
                                    .add_enabled(
                                        true,
                                        Button::new("Del").min_size([200.0, 100.0].into()),
                                    )
                                    .clicked()
                                {
                                    self.beatmap.timing_group.delete_timing(
                                        self.input_cache.select_timing_group,
                                        self.input_cache.select_timing_row.unwrap(),
                                    );
                                    self.input_cache.select_timing_row = None;
                                }
                            }
                            None => {
                                ui.add_enabled(
                                    false,
                                    Button::new("Del").min_size([200.0, 100.0].into()),
                                );
                            }
                        }
                    },
                    |ui| {
                        if ui
                            .add_enabled(
                                !same_time_with_timing,
                                Button::new("Add").min_size([200.0, 100.0].into()),
                            )
                            .clicked()
                        {
                            op.set(Some(Box::new(|this| {
                                if let Some(group) = this
                                    .beatmap
                                    .timing_group
                                    .timing_lines
                                    .get_mut(this.input_cache.select_timing_group)
                                {
                                    group.add_new(Timing::create_from_offset(
                                        this.input_cache.current_duration.as_millis() as OffsetType,
                                    ))
                                }
                            })));
                        }
                    },
                );
            });

        if let Some(op) = op.get_mut().take() {
            op(self)
        }
    }
}
