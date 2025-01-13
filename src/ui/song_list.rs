use crate::game::song::SongInfo;
use egui::{Color32, Response, ScrollArea, Ui};
use std::sync::Arc;

#[derive(Default)]
pub struct SongListUi {
    songs: Vec<Arc<SongInfo>>,
    /// The render y start to render the song
    ys: Vec<f32>,
    song_select: usize,
    beatmap_select: usize,
}

impl SongListUi {
    pub fn update_songs(&mut self, songs: Vec<Arc<SongInfo>>) {
        self.songs = songs;

        self.ys = Vec::with_capacity(self.songs.len());
    }

    pub fn songs(&self) -> &Vec<Arc<SongInfo>> {
        &self.songs
    }
}

impl egui::Widget for &mut SongListUi {
    fn ui(self, ui: &mut Ui) -> Response {
        ScrollArea::new([false, true])
            .auto_shrink(false)
            .max_height(f32::INFINITY)
            .show_viewport(ui, |ui, rect| {
                let scrolled = rect.min.y;
                let raw_available_rect = ui.available_rect_before_wrap();

                let height = raw_available_rect.max.y - raw_available_rect.min.y;
                let half_height = height / 2.0;

                if scrolled < half_height {
                    ui.scroll_with_delta((0.0, -(half_height - scrolled)).into());
                }

                ui.allocate_space((0.0, height).into());

                {
                    // draw background
                    let mut rect = raw_available_rect;
                    rect.min.y += scrolled;
                    rect.max.y += scrolled;
                    ui.painter().rect_filled(rect, 0.0, Color32::from_gray(9));
                }
                // ui.painter_at(rect).rect_filled(ui.available_rect_before_wrap(), 0.0, Color32::from_gray(9));

                for x in &self.songs {}

                for i in 0..100 {
                    let rect = ui.selectable_label(false, format!("Label{i}")).rect;
                }

                let scrolled = rect.min.y;
                ui.response()
            }).inner
    }
}