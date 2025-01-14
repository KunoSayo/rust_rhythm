use crate::game::OffsetType;
use egui::NumExt;
use serde::{Deserialize, Serialize};
use std::convert::Into;
use std::num::NonZeroU8;

// Store bpm with 100 times
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Bpm(u32);

impl From<f32> for Bpm {
    fn from(value: f32) -> Self {
        Self {
            0: (value * 100.0).round().at_least(1.0) as u32,
        }
    }
}

impl From<f64> for Bpm {
    fn from(value: f64) -> Self {
        Self {
            0: (value * 100.0).round().at_least(1.0) as u32,
        }
    }
}


impl Into<f32> for Bpm {
    fn into(self) -> f32 {
        self.0 as f32 / 100.0
    }
}


#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Timing {
    pub bpm: Bpm,
    pub offset: OffsetType,
    pub time_signature: NonZeroU8,
}
pub const DEFAULT_TIMING: Timing = Timing {
    bpm: Bpm(60 * 100),
    offset: 0,
    time_signature: match NonZeroU8::new(4) {
        Some(e) => e,
        None => unreachable!()
    },
};

impl Default for Timing {
    fn default() -> Self {
        DEFAULT_TIMING
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TimingLine {
    pub timings: Vec<Timing>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimingGroup {
    pub timings: Vec<TimingLine>,
}

impl TimingLine {
    pub fn add_new(&mut self, timing: Timing) {
        self.timings.retain(|x| x.offset != timing.offset);
        self.timings.push(timing);
        self.timings.sort_by_key(|x| x.offset);
    }
}

impl TimingGroup {
    pub fn new() -> Self {
        Self {
            timings: vec![TimingLine::default()],
        }
    }

    /// Return the timing in the group by the offset.
    /// if there no such group, return None. otherwise return the timing or default timing.
    pub fn get_timing(&self, group_index: usize, offset: OffsetType) -> Option<Timing> {
        if let Some(tl) = self.timings.get(group_index) {
            let t = if tl.timings.is_empty() {
                DEFAULT_TIMING
            } else {
                match tl.timings.binary_search_by_key(&offset, |x| x.offset) {
                    Ok(idx) => tl.timings[idx],
                    Err(idx) => {
                        if idx == 0 {
                            tl.timings[0]
                        } else {
                            tl.timings[idx - 1]
                        }
                    }
                }
            };
            Some(t)
        } else {
            None
        }
    }
}