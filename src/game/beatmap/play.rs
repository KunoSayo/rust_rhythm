use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::GamePos;
use crate::game::note::{LongNote, NormalNote, Note, NoteExt, NoteHitType};
use crate::game::timing::{TimingGroup, TimingLine};
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
    pub note_y: f32,
    pub note_end_y: f32,
    start_result: Option<NoteResult>,
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
    pub fn new(note: T, note_y: f32, note_end_y: f32) -> Self {
        Self {
            note,
            note_y,
            note_end_y,
            start_result: None,
        }
    }
    #[inline]
    pub fn is_later_miss(&self, judge_times: &JudgeTimes, time: OffsetType) -> bool {
        time > self.note.get_end_time().unwrap_or(self.note.get_time()) + judge_times.bad
    }

    #[inline]
    pub fn is_early_miss(&self, judge_times: &JudgeTimes, time: OffsetType) -> bool {
        if self.note.get_end_time().is_none()
            && self.start_result.is_none()
            && time > self.note.get_time() + judge_times.bad
        {
            true
        } else {
            false
        }
    }
}

impl<T: Note> Note for PlayingNote<T> {
    fn get_x(&self) -> f32 {
        self.note.get_x()
    }

    fn get_width(&self) -> f32 {
        self.note.get_width()
    }

    fn get_time(&self) -> OffsetType {
        self.note.get_time()
    }

    fn get_end_time(&self) -> Option<OffsetType> {
        self.note.get_end_time()
    }

    fn get_note_type(&self) -> NoteHitType {
        self.note.get_note_type()
    }

    fn get_timing_group(&self) -> u8 {
        self.note.get_timing_group()
    }
}

/// Notes in the same timing group
pub struct TrackNotes<Note> {
    timings: TimingLine,
    pub play_area: VecDeque<PlayingNote<Note>>,
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
    pub fn tick(
        &mut self,
        ops: &PlayOptions,
        judge_times: &JudgeTimes,
        game_time: f32,
        mut callback: impl FnMut(&PlayingNote<T>, NoteResult),
    ) {
        let offset = secs_to_offset_type(game_time);

        // move pending to play area for some lag case.
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
        self.play_area.retain_mut(|x| {
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

    pub fn get_play_notes(&self) -> &VecDeque<PlayingNote<T>> {
        &self.play_area
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
    pub raw_file: SongBeatmapFile,
    pub(crate) ops: PlayOptions,
    judge: JudgeTimes,
    pub normal_notes: Vec<TrackNotes<NormalNote>>,
    pub long_notes: Vec<TrackNotes<LongNote>>,
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
        let judge = JudgeTimes::default();
        let ops = PlayOptions::default();

        file.normal_notes.sort_by_key(|x| x.time);
        file.long_notes.sort_by_key(|x| x.start_time);

        fn add_notes<T: Note + Copy>(notes: &[T], track: &mut Vec<TrackNotes<T>>, tg: &TimingGroup, view_time: f32) {
            for x in notes {
                if x.get_timing_group() as usize >= track.len() {
                    track.resize_with(x.get_timing_group() as usize + 1, || TrackNotes::default());
                }
                let start_y = tg.get_gameplay_y(
                    x.get_time(),
                    x.get_timing_group(),
                    view_time,
                );
                let end_y = tg.get_gameplay_y(
                    x.get_end_time_or_time(),
                    x.get_timing_group(),
                    view_time,
                );
                track[x.get_timing_group() as usize]
                    .pending.push_back(PlayingNote::new(*x, start_y, end_y));
            }
        }

        let mut normal_notes = vec![];
        add_notes(&file.normal_notes, &mut normal_notes, &file.timing_group, ops.default_view_time);
        let mut long_notes = vec![];
        add_notes(&file.long_notes, &mut long_notes, &file.timing_group, ops.default_view_time);

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
            ops,
            judge,
            normal_notes,
            long_notes,
            combo_counter: Default::default(),
        }
    }

    pub fn tick(&mut self, game_time: f32) {
        self.tick_track(game_time);
    }
}
