use crate::engine::StateData;
use crate::game::beatmap::MapRule;
use crate::state::editor::editor::BeatMapEditor;
use egui::{Color32, Frame, Widget};

impl BeatMapEditor {
    pub fn render_settings_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let none_select_label = |str| {
                    egui::Label::new(str).selectable(false)
                };
                let edit = |s| {
                    egui::TextEdit::singleline(s)
                        .frame(true)
                        .background_color(Color32::DARK_GREEN)
                };
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Title: "));
                        self.dirty |= edit(&mut self.beatmap.metadata.title).ui(ui).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Artist: "));
                        self.dirty |= edit(&mut self.beatmap.metadata.artist).ui(ui).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Creator: "));
                        self.dirty |= edit(&mut self.beatmap.metadata.creator).ui(ui).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Version: "));
                        self.dirty |= edit(&mut self.beatmap.metadata.version).ui(ui).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Source: "));
                        self.dirty |= edit(&mut self.beatmap.metadata.source).ui(ui).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Tags: "));
                        self.dirty |= edit(&mut self.beatmap.metadata.tags).ui(ui).changed();
                    });

                    self.dirty |= ui.radio_value(&mut self.beatmap.rule, MapRule::Falling, "Falling").changed();
                    self.dirty |= ui.radio_value(&mut self.beatmap.rule, MapRule::FourKey, "4K").changed();
                })
            });
    }
}