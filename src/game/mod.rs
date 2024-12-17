//! All the map use ms as time mark
//!
//! `SongManager` manage the songs, load all the songs list
//!
//! `SongMap` represent real playing map in the game.

pub type MsType = u32;
pub type OffsetType = MsType;

pub mod note;
pub mod song;
pub mod map;
pub mod timing;
