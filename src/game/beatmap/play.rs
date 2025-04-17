use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::GamePos;
use crate::game::note::{LongNote, NormalNote, Note, NoteExt, NoteHitType};
use crate::game::timing::{TimingGroup, TimingLine};
use crate::game::{secs_to_offset_type, OffsetType};
use egui::ahash::{HashMap, HashSet};
use rayon::iter::IntoParallelRefMutIterator;
use std::collections::VecDeque;
use std::fmt::Display;
use std::time::Instant;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NoteResult {
    Miss,
    Bad,
    Good,
    Great,
    Perfect,
}

#[derive(Copy, Clone)]
pub struct JudgeTimes {
    pub perfect: OffsetType,
    pub great: OffsetType,
    pub good: OffsetType,
    pub bad: OffsetType,
    pub miss: OffsetType,
}

impl JudgeTimes {
    pub(crate) fn get_result(&self, click_time: OffsetType, hit_time: OffsetType) -> NoteResult {
        let delta = (click_time - hit_time).abs();
        println!("Get result for delta {}", click_time - hit_time);
        match delta {
            _ if delta <= self.perfect => NoteResult::Perfect,
            _ if delta <= self.great => NoteResult::Great,
            _ if delta <= self.good => NoteResult::Good,
            _ if delta <= self.bad => NoteResult::Bad,
            _ => NoteResult::Miss,
        }
    }
}

#[derive(Copy, Clone)]
pub struct PlayOptions {
    pub default_view_time: f32,
}

impl Default for JudgeTimes {
    fn default() -> Self {
        Self {
            perfect: 30,
            great: 60,
            good: 100,
            bad: 150,
            miss: 200,
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
pub struct ScoreCounter {
    max_combo: usize,
    combo: usize,
}

pub struct PlayingNote<NoteType> {
    note: NoteType,
    pub note_idx: usize,
    pub note_y: f32,
    pub note_end_y: f32,
    pub start_result: Option<NoteResult>,
    // the pointer holding this note.
    holding: HashSet<u64>,
}

pub enum PlayingNoteType<'a> {
    Normal(&'a mut PlayingNote<NormalNote>),
    Long(&'a mut PlayingNote<LongNote>),
}

impl ScoreCounter {
    pub fn accept_result(&mut self, result: NoteResult) {
        if result == NoteResult::Miss {
            self.combo = 0;
        } else {
            self.combo += 1;
        }
        self.max_combo = self.combo.max(self.max_combo);
    }

    pub fn should_display(&self) -> bool {
        self.combo > 2
    }

    pub fn get_combo(&self) -> usize {
        self.combo
    }
}

impl<T: Note> PlayingNote<T> {
    pub fn new(note: T, note_idx: usize, note_y: f32, note_end_y: f32) -> Self {
        Self {
            note,
            note_idx,
            note_y,
            note_end_y,
            start_result: None,
            holding: Default::default(),
        }
    }
    /// note is later miss for the start time
    #[inline]
    pub fn is_later_miss(&self, judge_times: &JudgeTimes, time: OffsetType) -> bool {
        // we use miss for some lag cases.
        time > self.note.get_time() + judge_times.miss && self.start_result.is_none()
    }

    #[inline]
    pub fn is_early_miss(&self, judge_times: &JudgeTimes, time: OffsetType) -> bool {
        if self.note.get_end_time().is_none()
            && self.start_result.is_none()
            && time > self.note.get_time() + judge_times.bad
            && time <= self.note.get_time() + judge_times.miss
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

impl PlayingNoteType<'_> {
    fn get_note(&self) -> &dyn Note {
        match self {
            PlayingNoteType::Normal(x) => &x.note,
            PlayingNoteType::Long(x) => &x.note,
        }
    }

    fn is_not_started(&self) -> bool {
        match self {
            PlayingNoteType::Normal(x) => true,
            PlayingNoteType::Long(x) => x.start_result.is_none(),
        }
    }
}

impl Note for PlayingNoteType<'_> {
    fn get_x(&self) -> f32 {
        self.get_note().get_x()
    }

    fn get_width(&self) -> f32 {
        self.get_note().get_width()
    }

    fn get_time(&self) -> OffsetType {
        self.get_note().get_time()
    }

    fn get_end_time(&self) -> Option<OffsetType> {
        self.get_note().get_end_time()
    }

    fn get_note_type(&self) -> NoteHitType {
        self.get_note().get_note_type()
    }

    fn get_timing_group(&self) -> u8 {
        self.get_note().get_timing_group()
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
        mut callback: impl FnMut(&mut PlayingNote<T>, NoteResult),
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
                if x.get_end_time().is_some() {
                    x.start_result = Some(NoteResult::Miss);
                    true
                } else {
                    false
                }
            } else if let Some(r) = x.start_result {
                if x.get_end_time_or_time() <= offset {
                    callback(x, r);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });
    }

    pub fn get_play_notes(&self) -> &VecDeque<PlayingNote<T>> {
        &self.play_area
    }

    fn remove_play_note(&mut self, idx: usize) {
        self.play_area.retain(|x| x.note_idx != idx);
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
    pointers: HashMap<u64, GamePos>,
    pub combo_counter: ScoreCounter,
}

impl Gaming {
    fn tick_track(
        &mut self,
        game_time: f32,
        mut callback: Option<impl FnMut(PlayingNoteType, NoteResult)>,
    ) {
        for x in self.normal_notes.iter_mut() {
            x.tick(&self.ops, &self.judge, game_time, |note, result| {
                self.combo_counter.accept_result(result);
                if let Some(cb) = &mut callback {
                    cb(PlayingNoteType::Normal(note), result);
                }
            });
        }
        for x in self.long_notes.iter_mut() {
            x.tick(&self.ops, &self.judge, game_time, |note, result| {
                self.combo_counter.accept_result(result);
                if let Some(cb) = &mut callback {
                    cb(PlayingNoteType::Long(note), result);
                }
            });
        }
    }

    pub fn load_game(mut file: SongBeatmapFile) -> Self {
        let judge = JudgeTimes::default();
        let ops = PlayOptions::default();

        file.normal_notes.sort_by_key(|x| x.time);
        file.long_notes.sort_by_key(|x| x.start_time);

        fn add_notes<T: Note + Copy>(
            notes: &[T],
            track: &mut Vec<TrackNotes<T>>,
            tg: &TimingGroup,
            view_time: f32,
            cnt: &mut usize,
        ) {
            for x in notes {
                if x.get_timing_group() as usize >= track.len() {
                    track.resize_with(x.get_timing_group() as usize + 1, || TrackNotes::default());
                }
                let start_y = tg.get_gameplay_y(x.get_time(), x.get_timing_group(), view_time);
                let end_y =
                    tg.get_gameplay_y(x.get_end_time_or_time(), x.get_timing_group(), view_time);
                track[x.get_timing_group() as usize]
                    .pending
                    .push_back(PlayingNote::new(*x, *cnt, start_y, end_y));
                *cnt += 1;
            }
        }

        let mut normal_notes = vec![];
        let mut total_notes = 0;
        add_notes(
            &file.normal_notes,
            &mut normal_notes,
            &file.timing_group,
            ops.default_view_time,
            &mut total_notes,
        );
        let mut long_notes = vec![];
        add_notes(
            &file.long_notes,
            &mut long_notes,
            &file.timing_group,
            ops.default_view_time,
            &mut total_notes,
        );

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
            pointers: Default::default(),
            combo_counter: Default::default(),
        }
    }

    pub fn tick(
        &mut self,
        game_time: f32,
        callback: Option<impl FnMut(PlayingNoteType, NoteResult)>,
    ) {
        self.tick_track(game_time, callback);
    }

    /// return the note result,
    pub fn process_input(&mut self, input: GamePos, pointer: u64) -> Option<NoteResult> {
        let time_range = input.time - self.judge.bad..=input.time + self.judge.miss;
        let in_time_range = |time: OffsetType| time_range.contains(&time);

        // first, we only allow to click one note (long or single)
        use rayon::iter::*;
        let the_first_note_to_click = self
            .normal_notes
            .par_iter_mut()
            .flat_map(|x| x.play_area.par_iter_mut())
            .map(|x| PlayingNoteType::Normal(x))
            .chain(
                self.long_notes
                    .par_iter_mut()
                    .flat_map(|x| x.play_area.par_iter_mut().map(|x| PlayingNoteType::Long(x))),
            )
            .filter(|x| {
                x.is_x_in_range(input.x) && in_time_range(x.get_time()) && x.is_not_started()
            })
            .min_by(|a, b| {
                a.get_time().cmp(&b.get_time()).then(
                    (a.get_x() - input.x)
                        .abs()
                        .total_cmp(&(b.get_x() - input.x).abs()),
                )
            });

        let mut ret = None;
        if let Some(first_note) = the_first_note_to_click {
            match first_note {
                PlayingNoteType::Normal(note) => {
                    let result = self.judge.get_result(input.time, note.get_time());
                    note.start_result = Some(result);
                    let idx = note.note_idx;
                    let tg = note.get_timing_group() as usize;
                    self.normal_notes[tg].remove_play_note(idx);
                    self.combo_counter.accept_result(result);
                    ret = Some(result);
                }
                PlayingNoteType::Long(note) => {
                    let result = self.judge.get_result(input.time, note.get_time());
                    note.start_result = Some(result);
                    // let idx = note.note_idx;
                    // let tg = note.get_timing_group() as usize;
                    // we remove it when end.
                    // self.long_notes[tg].remove_play_note(idx);
                }
            };
        }
        self.pointers.insert(pointer, input);

        self.long_notes
            .par_iter_mut()
            .flat_map(|x| x.play_area.par_iter_mut())
            .filter(|x| x.is_x_in_range(input.x))
            .for_each(|playing_note| {
                playing_note.holding.insert(pointer);
            });

        ret
    }

    pub fn process_input_leave(
        &mut self,
        input: GamePos,
        pointer: u64,
    ) -> Option<(NoteResult, PlayingNoteType)> {
        self.pointers.remove(&pointer);
        let time_range = input.time - self.judge.bad..=input.time + self.judge.miss;

        use rayon::iter::*;
        self.long_notes
            .par_iter_mut()
            .flat_map(|x| x.play_area.par_iter_mut())
            .filter(|x| x.is_x_in_range(input.x))
            .for_each(|playing_note| {
                playing_note.holding.remove(&pointer);
                if playing_note.holding.is_empty() && playing_note.start_result.is_some() {
                    for (p, input) in &self.pointers {
                        if playing_note.is_x_in_range(input.x) {
                            playing_note.holding.insert(*p);
                        }
                    }

                    if playing_note.holding.is_empty() {
                        playing_note.start_result = Some(NoteResult::Miss);
                    }
                }
            });
        None
    }
}
