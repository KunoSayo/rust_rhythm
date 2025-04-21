use crate::game::{GameTimeType, OffsetType};
use egui::{Color32, NumExt};
use ron::extensions::Extensions;
use ron::Options;
use serde::{Deserialize, Serialize};
use std::convert::Into;
use std::fmt::{Display, Formatter};
use std::num::NonZeroU8;
use std::str::FromStr;
use std::sync::LazyLock;

/// Store bpm with 100 times
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Bpm(i32);

pub fn get_ron_options() -> Options {
    Options::default().with_default_extension(Extensions::all())
}

pub fn get_ron_options_for_implicit_some() -> Options {
    Options::default().with_default_extension(Extensions::IMPLICIT_SOME)
}

impl From<f32> for Bpm {
    fn from(value: f32) -> Self {
        Self {
            0: (value * 100.0).round().at_least(1.0) as i32,
        }
    }
}

impl Default for Bpm {
    fn default() -> Self {
        Self::from(60.0)
    }
}

impl FromStr for Bpm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.trim().split(".");
        let first = it.next().ok_or(())?.parse::<u32>().map_err(|_| ())?;
        let part = it
            .next()
            .map(|x| x.chars().take(2).collect::<String>().parse::<u32>())
            .unwrap_or(Ok(0))
            .map_err(|_| ())?;
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
    pub time: OffsetType,
    /// 小节细分索引，若为 0 则表示该 Beat
    pub index: u8,
    pub detail: u8,
    pub is_measure: bool,
}

impl Beat {
    pub const fn get_color(&self) -> Color32 {
        let beat = self;
        let color = if beat.index == 0 {
            Color32::from_gray(if beat.is_measure { 233 } else { 222 })
        } else {
            if self.detail % 3 == 0 {
                // 0   1   2   3
                // 0 1 2 3 4 5 6
            }

            Color32::DARK_RED
        };

        color
    }
}

/// The timing event
/// Reset bpm or
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timing {
    /// The bpm set by this timing
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "bpm")]
    pub set_bpm: Option<Bpm>,
    /// The speed set by this timing
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "speed")]
    pub set_speed: Option<f32>,
    pub offset: OffsetType,
    pub time_signature: NonZeroU8,
    #[serde(skip)]
    /// The bpm extended from last timing or this timing
    bpm: Bpm,
    /// The speed extended from last timing or this timing
    #[serde(skip)]
    speed: f32,
    /// The gameplay y if view seconds is 1
    #[serde(skip)]
    start_y: f32,
}

pub const DEFAULT_TIMING: Timing = Timing {
    set_bpm: Some(Bpm(60 * 100)),
    set_speed: Some(1.0),
    offset: 0,
    time_signature: match NonZeroU8::new(4) {
        Some(e) => e,
        None => unreachable!(),
    },
    bpm: Bpm(60 * 100),
    speed: 1.0,
    start_y: 0.0,
};

pub static DEFAULT_TIMING_LINE: LazyLock<TimingLine> = LazyLock::new(|| TimingLine {
    timings: vec![DEFAULT_TIMING],
});

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
    pub fn get_bpm(&self) -> Bpm {
        self.bpm
    }

    pub fn get_speed(&self) -> f32 {
        self.speed
    }
    /// Return the left beat (or self) at the time
    pub fn get_left_beat(&self, time: OffsetType, detail: u8) -> Beat {
        let delta = time - self.offset;

        // beat interval (ms) = 60 * 1000ms / (bpm / 100)
        //                    = 60 * 1000 * 100 / bpm
        // delta / interval (number)   = delta * bpm / 60 / 1000 / 100

        let number =
            (delta as i64 * self.bpm.0 as i64 / 6000000) as i32 - if delta < 0 { 1 } else { 0 };

        let is_measure = (number % self.time_signature.get() as i32) == 0;
        let beat_time = self.get_beat_time(number, 0, detail);
        let mut cur = Beat {
            number,
            index: 0,
            detail,
            is_measure,
            time: beat_time,
        };

        for i in 1..detail {
            let bt = self.get_beat_time(number, i, detail);
            if bt <= time {
                cur = Beat {
                    number,
                    index: i,
                    detail,
                    is_measure: false,
                    time: bt,
                };
            }
        }

        cur
    }

    pub fn get_beat_time(&self, number: i32, index: u8, detail: u8) -> OffsetType {
        // beat interval = 60s / bpm

        let detail_offset = index as i64 * 60 * 1000 * 100 / self.bpm.0 as i64 / detail as i64;
        let beat_offset = self.offset
            + (self.bpm.0 as i64 - 1
            + (number as i64)
            .checked_mul(60 * 1000 * 100)
            .expect("Get beat time overflow!"))
            / self.bpm.0 as i64;
        detail_offset
            .checked_add(beat_offset)
            .expect("Time Overflow")
    }

    pub fn get_next_beat_by_beat(&self, cur_beat: &Beat, detail: u8) -> Beat {
        if cur_beat.index + 1 == detail {
            Beat {
                number: cur_beat.number + 1,
                index: 0,
                detail,
                is_measure: ((cur_beat.number + 1) % self.time_signature.get() as i32) == 0,
                time: self.get_beat_time(cur_beat.number + 1, 0, detail),
            }
        } else {
            Beat {
                number: cur_beat.number,
                index: cur_beat.index + 1,
                detail,
                is_measure: false,
                time: self.get_beat_time(cur_beat.number, cur_beat.index + 1, detail),
            }
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

    /// Create timing set bpm.
    pub fn new(bpm: Bpm, offset: OffsetType, time_signature: NonZeroU8) -> Self {
        Self {
            set_bpm: Some(bpm),
            bpm,
            offset,
            time_signature,
            set_speed: None,
            speed: 1.0,
            start_y: 0.0,
        }
    }
}

impl TimingLine {
    pub fn add_new(&mut self, timing: Timing) {
        self.timings.retain(|x| x.offset != timing.offset);
        self.timings.push(timing);
        self.update();
    }

    pub fn update(&mut self) {
        self.timings.retain(|x| x.offset >= 0);
        self.timings.sort_by_key(|x| x.offset);
        let mut cur_bpm = Bpm::default();
        let mut cur_speed = 1.0;
        let mut last_start_y = 0.0;
        let mut last_offset = 0;
        for t in self.timings.iter_mut() {
            t.start_y = last_start_y + cur_speed * (t.offset - last_offset) as f32;
            if let Some(bpm) = t.set_bpm {
                cur_bpm = bpm;
            }
            if let Some(speed) = t.set_speed {
                cur_speed = speed;
            }
            t.bpm = cur_bpm;
            t.speed = cur_speed;
            last_start_y = t.start_y;
            last_offset = t.offset;
        }
    }

    /// Return the timing slice that first element offset is less equal than then given offset in this timing line.
    pub fn get_timings(&self, offset: OffsetType) -> &[Timing] {
        let t = if self.timings.is_empty() {
            &[DEFAULT_TIMING][..]
        } else {
            match self.timings.binary_search_by_key(&offset, |x| x.offset) {
                Ok(idx) => &self.timings[idx..],
                Err(idx) => {
                    if idx == 0 {
                        &self.timings[0..]
                    } else {
                        &self.timings[idx - 1..]
                    }
                }
            }
        };
        t
    }

    pub(crate) fn get_y(&self, time: OffsetType) -> f32 {
        let offset = time;
        let timing = &self.get_timings(offset)[0];
        timing.start_y + ((time - timing.offset) as f32 / 1000.0) * timing.get_speed()
    }
    pub(crate) fn get_y_f32(&self, time: f32) -> f32 {
        let offset = (time * 1000.0).floor() as OffsetType;
        let timing = &self.get_timings(offset)[0];
        timing.start_y + ((time - timing.offset as f32 / 1000.0) * timing.get_speed())
    }
}

impl TimingGroup {
    pub fn new() -> Self {
        Self {
            timing_lines: vec![TimingLine::default()],
        }
    }

    pub fn update(&mut self) {
        for x in &mut self.timing_lines {
            x.update();
        }
    }

    /// Return the timing slice that first element offset is less equal than the given offset in the group by the offset.
    /// if there no such group or timing, return the next timing slice or default timing.
    pub fn get_timing(&self, group_index: usize, offset: OffsetType) -> &[Timing] {
        if let Some(tl) = self.timing_lines.get(group_index) {
            tl.get_timings(offset)
        } else {
            &[DEFAULT_TIMING][..]
        }
    }

    /// Return (left beat, now beat, right beat)
    pub fn get_near_beat(
        &self,
        group_index: usize,
        offset: OffsetType,
        detail: u8,
    ) -> (Beat, Option<Beat>, Beat) {
        let mut it = self.get_beat_iterator(group_index, offset, detail);
        let mut now = None;
        let (left, right) = {
            let beat = it.next().unwrap();
            if beat.time == offset {
                now = Some(beat);
                let tl = self.get_timing(group_index, offset - 1);
                let left = tl[0].get_left_beat(offset - 1, detail);
                (left, it.next().unwrap())
            } else {
                // left right
                (beat, it.next().unwrap())
            }
        };

        (left, now, right)
    }

    pub fn get_timing_by_idx(&mut self, group_index: usize, idx: usize) -> Option<&mut Timing> {
        if let Some(tl) = self.timing_lines.get_mut(group_index) {
            tl.timings.get_mut(idx)
        } else {
            None
        }
    }

    pub fn delete_timing(&mut self, group_index: usize, row: usize) {
        if let Some(tl) = self.timing_lines.get_mut(group_index) {
            tl.timings.remove(row);
        }
    }

    pub fn has_timing(&self, group_index: usize, offset: OffsetType) -> bool {
        if let Some(tl) = self.timing_lines.get(group_index) {
            tl.timings
                .binary_search_by_key(&offset, |x| x.offset)
                .is_ok()
        } else {
            false
        }
    }

    pub fn get_gameplay_y(&self, time: OffsetType, timing_group: u8, view_secs: f32) -> f32 {
        let tl = if let Some(tl) = self.timing_lines.get(timing_group as usize) {
            tl
        } else {
            &DEFAULT_TIMING_LINE
        };
        tl.get_y(time) * view_secs
    }

    pub fn get_gameplay_y_game_time(&self, time: GameTimeType, timing_group: u8, view_secs: f32) -> f32 {
        let tl = if let Some(tl) = self.timing_lines.get(timing_group as usize) {
            tl
        } else {
            &DEFAULT_TIMING_LINE
        };
        tl.get_y_f32(time as f32) * view_secs as f32
    }

    pub fn get_beat_iterator(
        &self,
        group_index: usize,
        start_offset: OffsetType,
        detail: u8,
    ) -> TimingGroupBeatIterator {
        TimingGroupBeatIterator {
            last_timing: self.get_timing(group_index, start_offset),
            last_beat: Err(start_offset),
            detail,
        }
    }
}

pub struct TimingGroupBeatIterator<'a> {
    last_timing: &'a [Timing],
    /// The last beat or the start offset time
    last_beat: Result<Beat, OffsetType>,
    detail: u8,
}

impl<'a> Iterator for TimingGroupBeatIterator<'a> {
    type Item = Beat;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.last_beat {
            Ok(beat) => {
                let next_beat = self.last_timing[0].get_next_beat_by_beat(&beat, self.detail);
                if self.last_timing.len() > 1 && next_beat.time >= self.last_timing[1].offset {
                    self.last_timing = &self.last_timing[1..];

                    let beat =
                        self.last_timing[0].get_left_beat(self.last_timing[0].offset, self.detail);
                    beat
                } else {
                    next_beat
                }
            }
            Err(start_time) => self.last_timing[0].get_left_beat(start_time, self.detail),
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

#[cfg(test)]
mod test {
    use crate::game::timing::{Beat, Bpm, Timing, TimingGroup, TimingLine};
    use crate::game::OffsetType;
    use std::num::NonZeroU8;

    #[test]
    fn test_beat_iter() {
        let mut tg = TimingGroup::new();
        tg.timing_lines.clear();

        let mut tl = TimingLine::default();
        tl.timings.clear();
        tl.add_new(Timing::new(Bpm::from(60.0), 0, NonZeroU8::new(7).unwrap()));

        tg.timing_lines.push(tl);

        {
            let beats = tg.get_beat_iterator(0, 0, 1).take(8).collect::<Vec<_>>();
            for i in 0..7 {
                assert_eq!(
                    beats[i],
                    Beat {
                        number: i as i32,
                        index: 0,
                        detail: 1,
                        is_measure: i == 0,
                        time: (i * 1000) as OffsetType,
                    }
                )
            }
            assert_eq!(
                beats[7],
                Beat {
                    number: 7,
                    index: 0,
                    detail: 1,
                    is_measure: true,
                    time: 7000 as OffsetType,
                }
            )
        }
        {
            let beats = tg.get_beat_iterator(0, 0, 2).take(15).collect::<Vec<_>>();
            for i in 0..7 {
                for idx in 0..2 {
                    assert_eq!(
                        beats[i * 2 + idx],
                        Beat {
                            number: i as i32,
                            index: idx as u8,
                            detail: 2,
                            is_measure: i == 0 && idx == 0,
                            time: (i * 1000 + idx * 500) as OffsetType,
                        }
                    )
                }
            }
            assert_eq!(
                beats[14],
                Beat {
                    number: 7,
                    index: 0,
                    detail: 2,
                    is_measure: true,
                    time: 7000 as OffsetType,
                }
            )
        }
    }
    #[test]
    fn test_multi_tl() {
        let mut tg = TimingGroup::new();
        tg.timing_lines.clear();

        {
            let mut tl = TimingLine::default();
            tl.timings.clear();
            tl.add_new(Timing::new(Bpm::from(60.0), 0, NonZeroU8::new(4).unwrap()));
            tl.add_new(Timing::new(
                Bpm::from(60.0),
                1500,
                NonZeroU8::new(4).unwrap(),
            ));
            tg.timing_lines.push(tl);
        }
        assert_eq!(tg.timing_lines.len(), 1);
        assert_eq!(tg.timing_lines[0].timings.len(), 2);

        {
            let beats = tg.get_beat_iterator(0, 0, 2).take(6).collect::<Vec<_>>();
            for i in 0..3 {
                assert_eq!(
                    beats[i],
                    Beat {
                        number: (i / 2) as i32,
                        index: (i as u8) % 2,
                        detail: 2,
                        is_measure: i == 0,
                        time: (i * 500) as OffsetType,
                    }
                )
            }
            assert_eq!(
                beats[3],
                Beat {
                    number: 0,
                    index: 0,
                    detail: 2,
                    is_measure: true,
                    time: 1500 as OffsetType,
                }
            );
            assert_eq!(
                beats[4],
                Beat {
                    number: 0,
                    index: 1,
                    detail: 2,
                    is_measure: false,
                    time: 2000 as OffsetType,
                }
            );
            assert_eq!(
                beats[5],
                Beat {
                    number: 1,
                    index: 0,
                    detail: 2,
                    is_measure: false,
                    time: 2500 as OffsetType,
                }
            );
        }
    }
}
