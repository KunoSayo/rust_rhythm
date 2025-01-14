#![allow(unused)]

use std::collections::{HashMap, HashSet};
use std::mem::swap;

use winit::dpi::PhysicalPosition;
use winit::event::{Touch, TouchPhase};
use winit::keyboard::PhysicalKey;

#[derive(Debug, Clone)]
pub struct Pointer {
    id: u64,
    loc: PhysicalPosition<f64>,
    phase: TouchPhase,
}

impl From<Touch> for Pointer {
    fn from(touch: Touch) -> Self {
        Self {
            id: touch.id,
            loc: touch.location,
            phase: touch.phase,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RawInputData {
    pub points: HashMap<usize, Pointer>,
    pub pressing: HashSet<PhysicalKey>,
}

#[derive(Default)]
pub struct BakedInputs {
    pub cur_temp_input: RawInputData,
    pub last_frame_input: RawInputData,
    pub cur_frame_input: RawInputData,
    /// only swap in states.game tick
    pub cur_temp_game_input: RawInputData,
    /// only swap in states.game tick
    pub last_temp_game_input: RawInputData,
    pub points: HashMap<u64, Pointer>,
    pub pressed_any_cur_frame: usize,
}


impl BakedInputs {
    pub fn process(&mut self, pressed: &HashSet<PhysicalKey>, released: &HashSet<PhysicalKey>) {
        for key in pressed.iter() {
            self.cur_temp_input.pressing.insert(*key);
            self.cur_temp_game_input.pressing.insert(*key);
        }

        for key in released.iter() {
            if self.last_temp_game_input.pressing.contains(key) {
                self.cur_temp_game_input.pressing.remove(key);
            }
            if self.cur_frame_input.pressing.contains(key) {
                self.cur_temp_input.pressing.remove(key);
            }
        }
    }
    /// save current input to last
    /// make current temp input to current frame input
    pub(in crate::engine) fn swap_frame(&mut self) {
        //save current to last
        swap(&mut self.cur_frame_input, &mut self.last_frame_input);
        //clone for not lose temp info
        self.cur_frame_input = self.cur_temp_input.clone();

        self.pressed_any_cur_frame = self.cur_frame_input.pressing.iter()
            .filter(|k| !self.last_frame_input.pressing.contains(k))
            .count();
    }

    pub fn is_pressed(&self, keys: &[PhysicalKey]) -> bool {
        keys.iter().any(|k| !self.last_frame_input.pressing.contains(k))
            && keys.iter().all(|k| self.cur_frame_input.pressing.contains(k))
    }
}

impl RawInputData {
    #[allow(unused)]
    pub fn empty() -> Self {
        Self::default()
    }
}