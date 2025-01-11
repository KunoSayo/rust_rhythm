//! All the beatmap use ms as time mark
//!
//! `SongManager` manage the songs, load all the songs list
//!
//! `SongBeatMap` represent real playing beatmap in the game.

pub type MsType = u32;
pub type OffsetType = MsType;

pub mod note;
pub mod song;
pub mod beatmap;
pub mod timing;
