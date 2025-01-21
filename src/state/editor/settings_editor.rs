use crate::engine::StateData;
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
                        edit(&mut self.song_beatmap_file.metadata.title).ui(ui);
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Artist: "));
                        edit(&mut self.song_beatmap_file.metadata.artist).ui(ui);
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Creator: "));
                        edit(&mut self.song_beatmap_file.metadata.creator).ui(ui);
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Version: "));
                        edit(&mut self.song_beatmap_file.metadata.version).ui(ui);
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Source: "));
                        edit(&mut self.song_beatmap_file.metadata.source).ui(ui);
                    });
                    ui.horizontal(|ui| {
                        ui.add(none_select_label("Tags: "));
                        edit(&mut self.song_beatmap_file.metadata.tags).ui(ui);
                    });
                })
            });
    }
}