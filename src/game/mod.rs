//! All the beatmap use ms as time mark
//!
//! `SongManager` manage the songs, load all the songs list
//!
//! `SongBeatMap` represent real playing beatmap in the game.

use egui::{Rect, Vec2};

pub type MsType = i64;
pub type OffsetType = MsType;

pub mod note;
pub mod song;
pub mod beatmap;
pub mod timing;
pub mod render;

pub fn secs_to_offset_type(sec: f32) -> OffsetType {
    (sec * 1000.0).round() as OffsetType
}

pub fn get_play_rect(rect: Rect) -> Rect {
    let center_point = rect.center();
    
    let (half_x, half_y) = if rect.height() <= rect.width() {
        // expand to the top
        let half_y = rect.height() / 2.0 - 10.0;
        let half_x = half_y * 4.0 / 3.0;
        (half_x, half_y)
    } else {
        // expand to the left
        let half_x = rect.width() / 2.0 - 10.0;
        let half_y = half_x * 0.75;
        (half_x, half_y)
    };
    let rect = Rect {
        min: center_point - Vec2::new(half_x, half_y),
        max: center_point + Vec2::new(half_x, half_y),
    };
    rect
}