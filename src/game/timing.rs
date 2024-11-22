use crate::game::OffsetType;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU8;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Timing {
    pub bpm: f64,
    pub offset: OffsetType,
    pub detail: NonZeroU8,
}

impl Default for Timing {
    const fn default() -> Self {
        Self {
            bpm: 120.0,
            offset: 0,
            detail: NonZeroU8::new(4).unwrap(),
        }
    }
}

pub const DEFAULT_TIMING: Timing = Timing::default();

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