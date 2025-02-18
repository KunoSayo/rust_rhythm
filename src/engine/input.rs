#![allow(unused)]

use egui::{Ui, WidgetText};
use single_thread_cell::SingleThreadRefCell;
use std::collections::{HashMap, HashSet};
use std::mem::swap;
use std::str::FromStr;
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

#[derive(Default, Copy, Clone)]
pub struct MouseState {
    pub pos: PhysicalPosition<f32>,
    pub left_click: bool,
    pub last_left_click: bool,
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
    pub mouse_state: MouseState,
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

        self.pressed_any_cur_frame = self
            .cur_frame_input
            .pressing
            .iter()
            .filter(|k| !self.last_frame_input.pressing.contains(k))
            .count();
    }

    /// Get press state between frames.
    pub fn is_pressed(&self, keys: &[PhysicalKey]) -> bool {
        keys.iter()
            .any(|k| !self.last_frame_input.pressing.contains(k))
            && keys
                .iter()
                .all(|k| self.cur_frame_input.pressing.contains(k))
    }
}

impl RawInputData {
    #[allow(unused)]
    pub fn empty() -> Self {
        Self::default()
    }
}

pub struct UiThreadTextEditCache {
    editing: &'static str,
    pub text: String,
}

impl Default for UiThreadTextEditCache {
    fn default() -> Self {
        Self {
            editing: "",
            text: "".to_string(),
        }
    }
}

static EDIT_CACHE: once_cell::sync::Lazy<SingleThreadRefCell<UiThreadTextEditCache>> =
    once_cell::sync::Lazy::new(|| Default::default());

pub fn get_edit_cache() -> single_thread_cell::SingleThreadRefMut<'static, UiThreadTextEditCache> {
    EDIT_CACHE.borrow_mut()
}

impl UiThreadTextEditCache {
    pub fn edit(&mut self, str: &str, id: &'static str) {
        if !self.is_editing(id) {
            self.editing = id;
            self.text = str.to_string();
        }
    }

    pub fn release(&mut self) {
        self.editing = "";
    }

    pub fn is_editing(&self, id: &'static str) -> bool {
        std::ptr::addr_eq(self.editing, id)
    }
}

/// Edit the text and return the edit result if end edit.
pub fn edit_dyn_data(ui: &mut Ui, id: &'static str, mut data_str: String) -> Option<String> {
    let mut cache = get_edit_cache();

    let response = if cache.is_editing(id) {
        ui.text_edit_singleline(&mut cache.text)
    } else {
        ui.text_edit_singleline(&mut data_str)
    };

    if response.has_focus() {
        cache.edit(&data_str, id);
        None
    } else if response.lost_focus() {
        cache.release();
        Some(cache.text.clone())
    } else {
        None
    }
}


pub fn optional_set<T>(ui: &mut Ui, text: impl Into<WidgetText>, data: &mut Option<T>, def: T) -> bool {
    let mut dirty = false;
    let mut set = data.is_some();
    dirty |= ui.checkbox(&mut set, text).changed();
    if set {
        data.get_or_insert(def);
    } else {
        data.take();
    }
    dirty
}

/// Optional set data
pub fn optional_edit<T: Default + FromStr + ToString>(
    ui: &mut Ui,
    id: &'static str,
    text: impl Into<WidgetText>,
    data: &mut Option<T>,
    def: T
) -> bool {
    let mut dirty = optional_set(ui, text, data, def);
    if let Some(v) = data {
        if let Some(result) = edit_dyn_data(ui, id, v.to_string()) {
            if let Ok(new) = result.parse::<T>() {
                data.insert(new);
                dirty |= true;
            }
        }
    }
    dirty
}
