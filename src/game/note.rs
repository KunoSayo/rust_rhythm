use crate::game::OffsetType;
use serde::{Deserialize, Serialize};

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

}

#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Clone, Copy, Debug)]
pub struct LongNote {
    pub x: f32,
    pub width: f32,
    pub start_time: OffsetType,
    pub end_time: OffsetType,
    pub note_type: NoteHitType,

}

pub trait Note {
    fn get_x(&self) -> f32;

    /// The full width. so note in the screen is [x - width / 2, x + width / 2]
    fn get_width(&self) -> f32;
    
    // Get the start time
    fn get_time(&self) -> OffsetType;

    fn get_note_type(&self) -> NoteHitType;
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

    fn get_note_type(&self) -> NoteHitType {
        self.note_type
    }
}