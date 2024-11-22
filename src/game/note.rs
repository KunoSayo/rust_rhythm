use crate::game::OffsetType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Ord, PartialOrd, PartialEq, Clone, Copy, Debug, Eq, Hash)]
pub enum NoteType {
    /// Click type
    Click,
    Slide,
}

#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Clone, Copy, Debug)]
pub struct NormalNote {
    pub x: f32,
    pub time: OffsetType,
    pub note_type: NoteType,

}

pub trait Note {
    fn get_x(&self) -> f32;

    fn get_time(&self) -> u32;

    fn get_note_type(&self) -> NoteType;
}

impl Note for NormalNote {
    fn get_x(&self) -> f32 {
        self.x
    }

    fn get_time(&self) -> u32 {
        self.time
    }

    fn get_note_type(&self) -> NoteType {
        self.note_type
    }
}