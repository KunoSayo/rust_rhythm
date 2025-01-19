use crate::engine::StateData;
use crate::state::editor::editor::BeatMapEditor;
use egui_extras::Column;

impl BeatMapEditor {
    pub fn render_timing_editor(&mut self, s: &mut StateData, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let table = egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto())
                    .column(Column::remainder());
            });
    }
}