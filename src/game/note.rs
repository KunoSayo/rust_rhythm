use crate::game::OffsetType;
use serde::{Deserialize, Serialize};

pub mod consts {
    pub const NOTE_HEIGHT_PIXEL: f32 = 32.0;
}
#[derive(Serialize, Deserialize, Ord, PartialOrd, PartialEq, Clone, Copy, Debug, Eq, Hash)]
pub enum NoteHitType {
    /// Click type
    Click,
    Slide,
}

#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Clone, Copy, Debug)]
pub struct NormalNote {
    pub x: f32,
    pub width: f32,
    pub time: OffsetType,
    pub note_type: NoteHitType,
    #[serde(default)]
    pub timing_group: u8,
}

#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Clone, Copy, Debug)]
pub struct LongNote {
    pub x: f32,
    pub width: f32,
    pub start_time: OffsetType,
    pub end_time: OffsetType,
    #[serde(default)]
    pub timing_group: u8,
}

pub trait Note {
    fn get_x(&self) -> f32;

    /// The full width. so note in the screen is [x - width / 2, x + width / 2]
    fn get_width(&self) -> f32;

    // Get the start time
    fn get_time(&self) -> OffsetType;
    
    fn get_end_time(&self) -> Option<OffsetType> { None }

    fn get_note_type(&self) -> NoteHitType;

    fn get_timing_group(&self) -> u8;
}

impl Note for NormalNote {
    fn get_x(&self) -> f32 {
        self.x
    }

    fn get_width(&self) -> f32 {
        self.width
    }

    fn get_time(&self) -> OffsetType {
        self.time
    }

    fn get_note_type(&self) -> NoteHitType {
        self.note_type
    }

    fn get_timing_group(&self) -> u8 {
        self.timing_group
    }
}

impl Note for LongNote {
    fn get_x(&self) -> f32 {
        self.x
    }

    fn get_width(&self) -> f32 {
        self.width
    }

    fn get_time(&self) -> OffsetType {
        self.start_time
    }

    fn get_end_time(&self) -> Option<OffsetType> {
        Some(self.end_time)
    }

    fn get_note_type(&self) -> NoteHitType {
        NoteHitType::Click
    }

    fn get_timing_group(&self) -> u8 {
        self.timing_group
    }
}

pub trait NoteExt {
    fn get_end_time_or_time(&self) -> OffsetType;
}

impl <T: Note> NoteExt for T {
    fn get_end_time_or_time(&self) -> OffsetType {
        self.get_end_time().unwrap_or(self.get_time())
    }
}