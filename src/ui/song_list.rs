use crate::game::song::SongInfo;
use egui::{Response, ScrollArea, Ui};

pub struct SongListUI {
    pub songs: Vec<SongInfo>,
}

impl egui::Widget for &SongListUI {
    fn ui(self, ui: &mut Ui) -> Response {
        ScrollArea::new([false, true])
            .show_rows(ui, 200.0, self.songs.len(), |ui, range| {
                ui.response()
            }).inner
    }
}