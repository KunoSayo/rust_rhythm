use crate::game::OffsetType;
use egui::NumExt;
use serde::{Deserialize, Serialize};
use std::convert::Into;
use std::num::NonZeroU8;

// Store bpm with 100 times
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Bpm(i32);

impl From<f32> for Bpm {
    fn from(value: f32) -> Self {
        Self {
            0: (value * 100.0).round().at_least(1.0) as i32,
        }
    }
}

impl From<f64> for Bpm {
    fn from(value: f64) -> Self {
        Self {
            0: (value * 100.0).round().at_least(1.0) as i32,
        }
    }
}


impl Into<f32> for Bpm {
    fn into(self) -> f32 {
        self.0 as f32 / 100.0
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub struct Beat {
    number: i32,
    is_measure: bool,
    time: OffsetType,
}

/// The timing event
/// Reset bpm or 
#[derive(Clone, Debug, Serialize, Deserialize)]
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

impl Timing {
    /// Return the left beat (or self) at the time
    pub fn get_left_beat(&self, time: OffsetType) -> Beat {
        let delta = time - self.offset;

        // beat interval (ms) = 60 * 1000ms / (bpm / 100)
        //                    = 60 * 1000 * 100 / bpm
        // delta / interval (number)   = delta * bpm / 60 / 1000 / 1000

        let number = (delta as i64 * self.bpm.0 as i64 / 60 / 1000 / 1000) as i32;
        let is_measure = (number % self.time_signature.get() as i32) == 0;
        let beat_time = self.get_beat_time(number);
        Beat {
            number,
            is_measure,
            time: beat_time,
        }
    }

    pub fn get_beat_time(&self, number: i32) -> OffsetType {
        self.offset + number * 60 * 1000 * 100 / self.bpm.0
    }

    pub fn get_next_beat_by_beat(&self, cur_beat: &Beat) -> Beat {
        Beat {
            number: cur_beat.number + 1,
            is_measure: ((cur_beat.number + 1) % self.time_signature.get() as i32) == 0,
            time: self.get_beat_time(cur_beat.number + 1),
        }
    }

    pub fn is_same_by_addr(&self, other: &Timing) -> bool {
        std::ptr::addr_eq(self as _, other as _)
    }
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
    /// if there no such group or something else, return the timing or default timing.
    pub fn get_timing(&self, group_index: usize, offset: OffsetType) -> &Timing {
        if let Some(tl) = self.timings.get(group_index) {
            let t = if tl.timings.is_empty() {
                &DEFAULT_TIMING
            } else {
                match tl.timings.binary_search_by_key(&offset, |x| x.offset) {
                    Ok(idx) => &tl.timings[idx],
                    Err(idx) => {
                        if idx == 0 {
                            &tl.timings[0]
                        } else {
                            &tl.timings[idx - 1]
                        }
                    }
                }
            };
            t
        } else {
            &DEFAULT_TIMING
        }
    }

    pub fn get_beat_iterator(&self, group_index: usize, start_offset: OffsetType) -> TimingGroupBeatIterator {
        TimingGroupBeatIterator {
            timing_group: self,
            last_timing: self.get_timing(group_index, start_offset),
            group_idx: group_index,
            last_beat: Err(start_offset),
        }
    }
}

pub struct TimingGroupBeatIterator<'a> {
    timing_group: &'a TimingGroup,
    last_timing: &'a Timing,
    group_idx: usize,
    /// The last beat or the start offset time
    last_beat: Result<Beat, OffsetType>,
}

impl<'a> Iterator for TimingGroupBeatIterator<'a> {
    type Item = Beat;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.last_beat {
            Ok(beat) => {
                let next_beat = self.last_timing.get_next_beat_by_beat(&beat);
                let maybe_next_timing = self.timing_group.get_timing(self.group_idx, next_beat.time);
                if self.last_timing.is_same_by_addr(maybe_next_timing) {
                    next_beat
                } else {
                    self.last_timing = maybe_next_timing;
                    self.last_timing.get_left_beat(next_beat.time)
                }
            }
            Err(start_time) => {
                self.last_timing.get_left_beat(start_time)
            }
        };

        self.last_beat = Ok(result);
        Some(result)
    }
}