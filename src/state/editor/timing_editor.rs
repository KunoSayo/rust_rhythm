use crate::engine::StateData;
use crate::state::editor::editor::BeatMapEditor;
use egui::panel::Side;
use egui::{Button, Frame};
use egui_extras::Column;

impl BeatMapEditor {
    pub fn render_timing_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
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
                            let mut op: std::cell::Cell<Option<Box<dyn FnOnce(&mut Self)>>> = Default::default();
                            for (idx, _) in self.song_beatmap_file.timing_group.timings.iter().enumerate() {
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
                                                    this.song_beatmap_file.timing_group.timings.remove(idx);
                                                })));
                                            }
                                        });
                                    });
                            }
                            if let Some(op) = op.get_mut().take() {
                                op(self)
                            }

                            ui.centered_and_justified(|ui| {
                                if ui.add(Button::new("+")).clicked() {
                                    self.song_beatmap_file.timing_group.timings.push(Default::default());
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

                let table = egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
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
                    body.rows(text_height, 0, |mut row| {
                        let idx = row.index();
                    })
                });
            });
    }
}