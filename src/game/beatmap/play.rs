use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::GamePos;
use crate::game::note::{LongNote, NormalNote, Note};
use crate::game::timing::TimingLine;
use crate::game::{secs_to_offset_type, OffsetType};
use std::collections::VecDeque;
use std::fmt::Display;
use std::time::Instant;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NoteResult {
    Miss,
    Bad,
    Great,
    Perfect,
}

#[derive(Copy, Clone)]
pub struct JudgeTimes {
    pub perfect: OffsetType,
    pub great: OffsetType,
    pub bad: OffsetType,
    pub miss: OffsetType,
}

#[derive(Copy, Clone)]
pub struct PlayOptions {
    pub default_view_time: f32,
}

impl Default for JudgeTimes {
    fn default() -> Self {
        Self {
            perfect: 10,
            great: 20,
            bad: 30,
            miss: 40,
        }
    }
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            default_view_time: 1.0,
        }
    }
}

#[derive(Default)]
pub struct ComboCounter {
    combo: usize,
}

pub struct PlayingNote<NoteType> {
    note: NoteType,
    start_result: Option<NoteResult>,
}

impl<T: Note> From<T> for PlayingNote<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl ComboCounter {
    pub fn accept_result(&mut self, result: NoteResult) {
        if result == NoteResult::Miss {
            self.combo = 0;
        } else {
            self.combo += 1;
        }
    }

    pub fn should_display(&self) -> bool {
        self.combo > 2
    }

    pub fn get_combo(&self) -> usize {
        self.combo
    }
}

impl<T: Note> PlayingNote<T> {
    pub fn new(note: T) -> Self {
        Self {
            note,
            start_result: None,
        }
    }
    #[inline]
    pub fn is_later_miss(&self, judge_times: &JudgeTimes, time: OffsetType) -> bool {
        time > self.note.get_end_time().unwrap_or(self.note.get_time()) + judge_times.bad
    }

    #[inline]
    pub fn is_early_miss(&self, judge_times: &JudgeTimes, time: OffsetType) -> bool {
        if self.note.get_end_time().is_none() && self.start_result.is_none() && time > self.note.get_time() + judge_times.bad {
            true
        } else {
            false
        }
    }
}

/// Notes in the same timing group
pub struct TrackNotes<Note> {
    timings: TimingLine,
    play_area: VecDeque<PlayingNote<Note>>,
    pending: VecDeque<PlayingNote<Note>>,
}

impl<T> Default for TrackNotes<T> {
    fn default() -> Self {
        Self {
            timings: Default::default(),
            play_area: Default::default(),
            pending: Default::default(),
        }
    }
}
impl<T: Note> TrackNotes<T> {
    pub fn tick(&mut self, ops: &PlayOptions, judge_times: &JudgeTimes, game_time: f32, mut callback: impl FnMut(&PlayingNote<T>, NoteResult)) {
        let offset = secs_to_offset_type(game_time);
        
        while let Some(note) = self.pending.front() {
            if note.note.get_time() <= secs_to_offset_type(game_time + ops.default_view_time + 1.0)
            {
                unsafe {
                    // SAFETY: we just got the front.
                    self.play_area
                        .push_back(self.pending.pop_front().unwrap_unchecked());
                }
            } else {
                break;
            }
        }
        self.play_area
            .retain_mut(|x| {
                if x.is_later_miss(&judge_times, offset) {
                    callback(x, NoteResult::Miss);
                    false
                } else if x.is_early_miss(&judge_times, offset) {
                    x.start_result = Some(NoteResult::Miss);
                    callback(x, NoteResult::Miss);
                    true
                } else {
                    true
                }
            });
    }
}

pub struct GamingInput {
    time: Instant,
    pos: GamePos,
}

impl GamingInput {
    pub fn new(time: Instant, pos: GamePos) -> Self {
        Self { time, pos }
    }

    pub fn get_game_time(
        &self,
        now: Instant,
        music_start_time: Instant,
        music_time: f32,
    ) -> OffsetType {
        let delta = now.duration_since(self.time).as_secs_f32();
        let delta_to_start = now.duration_since(music_start_time).as_secs_f32();
        if delta_to_start < delta {
            // the input is pressed when the music is not playing
            OffsetType::MIN
        } else {
            secs_to_offset_type(music_time - delta)
        }
    }
}

pub struct Gaming {
    raw_file: SongBeatmapFile,
    ops: PlayOptions,
    judge: JudgeTimes,
    normal_notes: Vec<TrackNotes<NormalNote>>,
    long_notes: Vec<TrackNotes<LongNote>>,
    combo_counter: ComboCounter,
}

impl Gaming {
    fn tick_track(&mut self, game_time: f32) {
        for x in self.normal_notes.iter_mut() {
            x.tick(&self.ops, &self.judge, game_time, |note, result| {
                self.combo_counter.accept_result(result);
            });
        }
        for x in self.long_notes.iter_mut() {
            x.tick(&self.ops, &self.judge, game_time, |note, result| {
                self.combo_counter.accept_result(result);
            });
        }
    }

    pub fn load_game(mut file: SongBeatmapFile) -> Self {
        file.normal_notes.sort_by_key(|x| x.time);
        file.long_notes.sort_by_key(|x| x.start_time);
        let mut normal_notes = vec![];
        for x in &file.normal_notes {
            if x.timing_group as usize >= normal_notes.len() {
                normal_notes.resize_with(x.timing_group as usize + 1, || TrackNotes::default());
            }
            normal_notes[x.timing_group as usize]
                .pending
                .push_back(PlayingNote::new(*x));
        }
        let mut long_notes = vec![];
        for x in &file.long_notes {
            if x.timing_group as usize >= long_notes.len() {
                long_notes.resize_with(x.timing_group as usize + 1, || TrackNotes::default());
            }
            long_notes[x.timing_group as usize]
                .pending
                .push_back(PlayingNote::new(*x));
        }

        for (idx, tl) in file.timing_group.timing_lines.iter().enumerate() {
            if let Some(n) = normal_notes.get_mut(idx) {
                n.timings = tl.clone();
            }
            if let Some(n) = long_notes.get_mut(idx) {
                n.timings = tl.clone();
            }
        }

        Self {
            raw_file: file.clone(),
            ops: Default::default(),
            judge: JudgeTimes::default(),
            normal_notes,
            long_notes,
            combo_counter: Default::default(),
        }
    }

    pub fn tick(&mut self, game_time: f32) {
        self.tick_track(game_time);
    }
}
