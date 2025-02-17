//! All the beatmap use ms as time mark
//!
//! `SongManager` manage the songs, load all the songs list
//!
//! `SongBeatMap` represent real playing beatmap in the game.

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