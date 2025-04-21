//! All the beatmap use ms as time mark
//!
//! `SongManager` manage the songs, load all the songs list
//!
//! `SongBeatMap` represent real playing beatmap in the game.

use egui::{NumExt, Rect, Vec2};

pub type MsType = i64;
pub type OffsetType = MsType;
/// The game time in seconds.
pub type GameTimeType = f64;

pub mod note;
pub mod song;
pub mod beatmap;
pub mod timing;
pub mod render;

#[inline]
#[must_use]
pub fn secs_to_offset_type(sec: impl Into<f64>) -> OffsetType {
    (sec.into() * 1000.0).round() as OffsetType
}

#[inline]
#[must_use]
pub fn offset_type_to_secs(offset: OffsetType) -> GameTimeType {
    offset as GameTimeType / 1000.0
}

pub fn get_play_rect(rect: Rect) -> Rect {
    let center_point = rect.center();
    // 4:3 play area.
    
    // if 100x75 => we expand top
    // if 100x10 => we expand top
    // if 100x200=> we expand left
    let (half_x, half_y) = if rect.height() * 3.0 <= rect.width() * 4.0 {
        // expand to the top
        let half_y = (rect.height() / 2.0 - 10.0).at_least(0.0);
        let half_x = half_y * 4.0 / 3.0;
        (half_x, half_y)
    } else {
        // expand to the left
        let half_x = (rect.width() / 2.0 - 10.0).at_least(0.0);
        let half_y = half_x * 0.75;
        (half_x, half_y)
    };
    let rect = Rect {
        min: center_point - Vec2::new(half_x, half_y),
        max: center_point + Vec2::new(half_x, half_y),
    };
    rect
}