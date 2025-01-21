use crate::game::beatmap::SongBeatmapInfo;
use crate::game::song::SongInfo;
use egui::{Button, Color32, NumExt, RichText, ScrollArea, Ui, Vec2};
use std::cell::Cell;
use std::sync::Arc;

#[derive(Default)]
pub struct SongListUi {
    allow_select_song: bool,
    songs: Vec<Arc<SongInfo>>,

    song_select: Cell<usize>,
    beatmap_select: Cell<usize>,
}


const SONG_BAR_HEIGHT: f32 = 200.0;
const BEATMAP_BAR_HEIGHT: f32 = 100.0;


pub struct EnterResult {
    pub song: Arc<SongInfo>,
    pub beatmap: Option<SongBeatmapInfo>,
}

#[derive(Default)]
pub struct SongListUiResponse {
    pub result: Option<EnterResult>,
}

impl SongListUi {
    pub fn update_songs(&mut self, songs: Vec<Arc<SongInfo>>) {
        self.songs = songs;
    }

    pub fn songs(&self) -> &Vec<Arc<SongInfo>> {
        &self.songs
    }

    pub fn render_beatmap(&self, ui: &mut Ui, song_idx: usize, idx: usize) -> Option<EnterResult> {
        let mut result = None;
        let beatmap = &self.songs[song_idx].maps[idx];


        #[cfg(debug_assertions)]
        let old_y = ui.next_widget_position().y;

        let button = Button::new(beatmap.song_beatmap_file.get_show_name())
            .fill(if self.beatmap_select.get() == idx {
                Color32::BLUE
            } else {
                Color32::DARK_BLUE
            });

        let song_width = if self.beatmap_select.get() == idx {
            250.0
        } else {
            150.0
        };


        let response = ui.add_sized(Vec2::new(song_width, 97.0), button);
        #[cfg(debug_assertions)]
        {
            let now_y = ui.next_widget_position().y;
            debug_assert!((now_y - old_y - BEATMAP_BAR_HEIGHT) < 1e-4);
        }

        if response.clicked() {
            if self.beatmap_select.get() == idx {
                result = Some(EnterResult {
                    song: self.songs[song_idx].clone(),
                    beatmap: Some(beatmap.clone()),
                });
            }
            self.beatmap_select.set(idx);
        }

        result
    }

    pub fn render_song(&self, ui: &mut Ui, idx: usize) -> Option<EnterResult> {
        let song = &self.songs[idx];

        let mut result = None;

        #[cfg(debug_assertions)]
        let old_y = ui.next_widget_position().y;

        let selected = self.song_select.get() == idx;
        let button = Button::new(RichText::new(&song.title).color(Color32::WHITE))
            .selected(selected)
            .fill(if self.song_select.get() == idx {
                Color32::LIGHT_BLUE
            } else {
                Color32::DARK_GRAY
            });

        let song_width = if self.song_select.get() == idx {
            300.0
        } else {
            200.0
        };

        let response = ui.add_sized(Vec2::new(song_width, 197.0), button);


        #[cfg(debug_assertions)]
        {
            let now_y = ui.next_widget_position().y;
            debug_assert!((now_y - old_y - SONG_BAR_HEIGHT) < 1e-4);
        }

        if self.song_select.get() == idx {
            for map_idx in 0..song.maps.len() {
                if let Some(r) = self.render_beatmap(ui, idx, map_idx) {
                    result = Some(r);
                }
            }
        }
        if response.clicked() && result.is_none() {
            if self.song_select.get() == idx {
                let song = &self.songs[idx];
                result = Some(EnterResult {
                    song: song.clone(),
                    beatmap: None,
                });
            }
            self.song_select.set(idx);
            self.beatmap_select.set(song.maps.len() >> 1);
        }
        result
    }

    pub fn ui(&mut self, ui: &mut Ui) -> SongListUiResponse {
        let mut response = SongListUiResponse::default();
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

                let first_display_y = (scrolled - height).at_least(0.0);


                for idx in 0..self.songs.len() {
                    // render song bar

                    if let Some(result) = self.render_song(ui, idx) {
                        response.result = Some(result);
                    }
                }
                ui.allocate_space((0.0, height).into());

                let cur_y = ui.next_widget_position().y;

                let target_y = cur_y - half_height;
                if scrolled > target_y {
                    ui.scroll_with_delta((0.0, scrolled - target_y).into());
                }
            });
        response
    }
}