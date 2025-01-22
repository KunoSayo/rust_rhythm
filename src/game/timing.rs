use crate::game::OffsetType;
use egui::NumExt;
use serde::{Deserialize, Serialize};
use std::convert::Into;
use std::fmt::{Display, Formatter};
use std::num::NonZeroU8;
use std::str::FromStr;

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

impl FromStr for Bpm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.trim().split(".");
        let first = it.next().ok_or(())?.parse::<u32>().map_err(|_| ())?;
        let part = it.next().map(|x| x.chars().take(2).collect::<String>().parse::<u32>())
            .unwrap_or(Ok(0)).map_err(|_| ())?;
        Ok(Bpm {
            0: (first * 100) as i32 + part as i32,
        })
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
    pub number: i32,
    pub is_measure: bool,
    pub time: OffsetType,
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
    pub timing_lines: Vec<TimingLine>,
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

    pub fn create_from_offset(offset: OffsetType) -> Self {
        Self {
            offset,
            ..Default::default()
        }
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
            timing_lines: vec![TimingLine::default()],
        }
    }

    /// Return the timing slice that first element offset is less equal than the given offset in the group by the offset.
    /// if there no such group or timing, return the next timing slice or default timing.
    pub fn get_timings(&self, group_index: usize, offset: OffsetType) -> &[Timing] {
        if let Some(tl) = self.timing_lines.get(group_index) {
            let t = if tl.timings.is_empty() {
                &[DEFAULT_TIMING][..]
            } else {
                match tl.timings.binary_search_by_key(&offset, |x| x.offset) {
                    Ok(idx) => &tl.timings[idx..],
                    Err(idx) => {
                        if idx == 0 {
                            &tl.timings[0..]
                        } else {
                            &tl.timings[idx - 1..]
                        }
                    }
                }
            };
            t
        } else {
            &[DEFAULT_TIMING][..]
        }
    }

    pub fn get_timing_by_idx(&mut self, group_index: usize, idx: usize) -> Option<&mut Timing> {
        if let Some(tl) = self.timing_lines.get_mut(group_index) {
            tl.timings.get_mut(idx)
        } else {
            None
        }
    }

    pub fn has_timing(&self, group_index: usize, offset: OffsetType) -> bool {
        if let Some(tl) = self.timing_lines.get(group_index) {
            tl.timings.binary_search_by_key(&offset, |x| x.offset).is_ok()
        } else {
            false
        }
    }

    pub fn get_beat_iterator(&self, group_index: usize, start_offset: OffsetType) -> TimingGroupBeatIterator {
        TimingGroupBeatIterator {
            last_timing: self.get_timings(group_index, start_offset),
            last_beat: Err(start_offset),
        }
    }
}

pub struct TimingGroupBeatIterator<'a> {
    last_timing: &'a [Timing],
    /// The last beat or the start offset time
    last_beat: Result<Beat, OffsetType>,
}

impl<'a> Iterator for TimingGroupBeatIterator<'a> {
    type Item = Beat;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.last_beat {
            Ok(beat) => {
                let next_beat = self.last_timing[0].get_next_beat_by_beat(&beat);
                if self.last_timing.len() > 1 && next_beat.time >= self.last_timing[1].offset {
                    self.last_timing = &self.last_timing[1..];
                    self.last_timing[0].get_left_beat(self.last_timing[0].offset)
                } else {
                    next_beat
                }
            }
            Err(start_time) => {
                self.last_timing[0].get_left_beat(start_time)
            }
        };

        self.last_beat = Ok(result);
        Some(result)
    }
}

impl Display for Bpm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, self.0 % 100)
    }
}