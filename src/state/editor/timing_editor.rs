use crate::engine::StateData;
use crate::state::editor::editor::BeatMapEditor;

impl BeatMapEditor {
    pub fn render_timing_editor(&mut self, s: StateData, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .show(ctx, |ui| {});
    }
}