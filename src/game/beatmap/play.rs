use crate::game::beatmap::file::SongBeatmapFile;
use crate::game::beatmap::GamePos;
use crate::game::note::{LongNote, NormalNote, Note, NoteExt, NoteHitType};
use crate::game::timing::{TimingGroup, TimingLine};
use crate::game::{offset_type_to_secs, secs_to_offset_type, GameTimeType, OffsetType};
use egui::ahash::{HashMap, HashSet};
use std::collections::VecDeque;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NoteResult {
    Miss,
    Bad,
    Good,
    Great,
    Perfect,
}

impl NoteResult {
    pub fn is_miss(self) -> bool {
        self == NoteResult::Miss
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct NoteHitResult {
    pub grade: NoteResult,
    pub delta: OffsetType,
}

impl NoteHitResult {
    pub fn is_miss(&self) -> bool {
        self.grade.is_miss()
    }
}

impl NoteHitResult {
    pub fn new(grade: NoteResult, delta: OffsetType) -> Self {
        Self { grade, delta }
    }
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
    pub(crate) fn get_result(&self, click_time: OffsetType, hit_time: OffsetType) -> NoteHitResult {
        let delta = (click_time - hit_time).abs();
        // println!("Get result for delta {}", click_time - hit_time);
        let grade = match delta {
            _ if delta <= self.perfect => NoteResult::Perfect,
            _ if delta <= self.great => NoteResult::Great,
            _ if delta <= self.good => NoteResult::Good,
            _ if delta <= self.bad => NoteResult::Bad,
            _ => NoteResult::Miss,
        };
        NoteHitResult::new(grade, click_time - hit_time)
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

pub struct ScoreCounter {
    total_result: u32,
    max_combo: u32,
    combo: u32,
    result_map: HashMap<NoteResult, u32>,
    deltas: Vec<OffsetType>,
}

pub struct PlayingNote<NoteType> {
    note: NoteType,
    pub note_idx: usize,
    pub note_y: f32,
    pub note_end_y: f32,
    pub start_result: Option<NoteHitResult>,
    // the pointer holding this note.
    holding: HashSet<u64>,
}

pub enum PlayingNoteType<'a> {
    Normal(&'a mut PlayingNote<NormalNote>),
    Long(&'a mut PlayingNote<LongNote>),
}

impl ScoreCounter {
    pub fn accept_result(&mut self, result: NoteHitResult) {
        if result.grade == NoteResult::Miss {
            self.combo = 0;
        } else {
            self.combo += 1;
        }
        *self.result_map.get_mut(&result.grade).unwrap() += 1;
        self.deltas.push(result.delta);
        self.max_combo = self.combo.max(self.max_combo);
    }

    pub fn should_display(&self) -> bool {
        self.combo > 2
    }

    pub fn get_combo(&self) -> u32 {
        self.combo
    }

    pub fn get_deltas(&self) -> &Vec<OffsetType> {
        &self.deltas
    }
    pub fn get_note_count(&self, result: NoteResult) -> u32 {
        self.result_map[&result]
    }
    pub fn get_max_combo(&self) -> u32 {
        self.max_combo
    }
    pub fn new(total_result: u32) -> Self {
        let mut result_map = HashMap::default();
        result_map.insert(NoteResult::Miss, 0);
        result_map.insert(NoteResult::Bad, 0);
        result_map.insert(NoteResult::Good, 0);
        result_map.insert(NoteResult::Great, 0);
        result_map.insert(NoteResult::Perfect, 0);
        Self {
            total_result,
            max_combo: 0,
            combo: 0,
            result_map,
            deltas: Vec::with_capacity(total_result as usize),
        }
    }

    pub fn get_score(&self) -> u32 {
        if self.total_result == 0 {
            return 0;
        }
        let mx_score = 1_000_000;
        let mut result = 0;

        result += self.result_map[&NoteResult::Perfect] * mx_score / self.total_result;
        result += self.result_map[&NoteResult::Great] * mx_score / 2 / self.total_result;
        result += self.result_map[&NoteResult::Good] * mx_score / 4 / self.total_result;
        result += self.result_map[&NoteResult::Bad] * mx_score / 5 / self.total_result;

        result
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
            PlayingNoteType::Normal(_) => true,
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
    pub play_area: VecDeque<PlayingNote<Note>>,
    pending: VecDeque<PlayingNote<Note>>,
}

impl<T> Default for TrackNotes<T> {
    fn default() -> Self {
        Self {
            play_area: Default::default(),
            pending: Default::default(),
        }
    }
}
impl<T: Note> TrackNotes<T> {
    /// Tick the track.
    /// The callback will be called before the start result set.
    pub fn tick(
        &mut self,
        ops: &PlayOptions,
        judge_times: &JudgeTimes,
        game_time: GameTimeType,
        gameplay_y: f32,
        mut callback: impl FnMut(&mut PlayingNote<T>, NoteHitResult),
    ) {
        let offset = secs_to_offset_type(game_time);

        // move pending to play area for some lag case.
        while let Some(note) = self.pending.front() {
            if note.note.get_time() <= secs_to_offset_type(game_time + ops.default_view_time as GameTimeType + 1.0)
                || (note.note_y - gameplay_y).abs() < 2.0
                || (note.note_end_y - gameplay_y).abs() < 2.0
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
                callback(x, NoteHitResult::new(NoteResult::Miss, judge_times.miss));
                if x.get_end_time().is_some() {
                    x.start_result = Some(NoteHitResult::new(NoteResult::Miss, judge_times.miss));
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
pub struct Gaming {
    pub raw_file: SongBeatmapFile,
    pub(crate) ops: PlayOptions,
    judge: JudgeTimes,
    pub normal_notes: Vec<TrackNotes<NormalNote>>,
    pub long_notes: Vec<TrackNotes<LongNote>>,
    pointers: HashMap<u64, GamePos>,
    pub score_counter: ScoreCounter,
    pub auto_play: bool,
}

impl Gaming {
    fn tick_tracks(
        &mut self,
        game_time: GameTimeType,
        mut callback: Option<impl FnMut(PlayingNoteType, NoteHitResult)>,
    ) {
        for (x, tl) in self.normal_notes.iter_mut().zip(self.raw_file.timing_group.timing_lines.iter()) {
            let y = tl.get_y_f32(game_time as f32);
            if self.auto_play {
                while let Some(note) = x.play_area.front_mut() {
                    let delta = offset_type_to_secs(note.note.time) - game_time;
                    if delta <= 0.001 {
                        let result = NoteHitResult::new(NoteResult::Perfect, 0);
                        if let Some(cb) = &mut callback {
                            cb(PlayingNoteType::Normal(note), result);
                        }
                        self.score_counter.accept_result(result);

                        x.play_area.pop_front();
                    } else {
                        break;
                    }
                }
            }
            x.tick(&self.ops, &self.judge, game_time, y, |note, result| {
                self.score_counter.accept_result(result);
                if let Some(cb) = &mut callback {
                    cb(PlayingNoteType::Normal(note), result);
                }
            });
        }
        for (x, tl) in self.long_notes.iter_mut().zip(self.raw_file.timing_group.timing_lines.iter()) {
            let y = tl.get_y_f32(game_time as f32);
            if self.auto_play {
                x.play_area.retain_mut(|note| {
                    let delta = offset_type_to_secs(note.note.start_time) - game_time;
                    let end_delta = offset_type_to_secs(note.note.end_time) - game_time;
                    if delta <= 0.001 && note.start_result.is_none() {
                        let result = NoteHitResult::new(NoteResult::Perfect, 0);
                        if let Some(cb) = &mut callback {
                            cb(PlayingNoteType::Long(note), result);
                        }
                        note.start_result = Some(result);
                    }
                    if end_delta <= 0.001 {
                        let result = NoteHitResult::new(NoteResult::Perfect, 0);
                        self.score_counter.accept_result(result);
                        if let Some(cb) = &mut callback {
                            cb(PlayingNoteType::Long(note), result);
                        }
                        note.start_result = Some(result);
                        return false;
                    }
                    true
                });
            }
            x.tick(&self.ops, &self.judge, game_time, y, |note, result| {
                self.score_counter.accept_result(result);
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

        Self {
            raw_file: file.clone(),
            ops,
            judge,
            normal_notes,
            long_notes,
            pointers: Default::default(),
            score_counter: ScoreCounter::new(total_notes as u32),
            auto_play: false,
        }
    }

    pub fn tick(
        &mut self,
        game_time: GameTimeType,
        callback: Option<impl FnMut(PlayingNoteType, NoteHitResult)>,
    ) {
        self.tick_tracks(game_time, callback);
    }

    /// return the note hit result, and if it is long start.
    pub fn process_input(&mut self, input: GamePos, pointer: u64) -> Option<(NoteHitResult, bool)> {
        let time_range = input.time - self.judge.bad..=input.time + self.judge.miss;
        let long_time_range = input.time - self.judge.bad..=input.time + self.judge.bad;
        let in_time_range = |time: OffsetType| time_range.contains(&time);

        // first, we only allow to click one note (long or single)
        use rayon::iter::*;
        let the_first_note_to_click = self
            .normal_notes
            .par_iter_mut()
            .flat_map(|x| x.play_area.par_iter_mut())
            .filter(|x| in_time_range(x.note.time))
            .map(|x| PlayingNoteType::Normal(x))
            .chain(self.long_notes.par_iter_mut().flat_map(|x| {
                x.play_area
                    .par_iter_mut()
                    .filter(|x| long_time_range.contains(&x.note.start_time))
                    .map(|x| PlayingNoteType::Long(x))
            }))
            .filter(|x| x.is_x_in_range(input.x) && x.is_not_started())
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
                    self.score_counter.accept_result(result);
                    ret = Some((result, false));
                }
                PlayingNoteType::Long(note) => {
                    let result = self.judge.get_result(input.time, note.get_time());
                    note.start_result = Some(result);
                    ret = Some((result, true));
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

    pub fn is_end(&self) -> bool {
        self.normal_notes
            .iter()
            .all(|x| x.pending.is_empty() && x.play_area.is_empty())
            && self
                .long_notes
                .iter()
                .all(|x| x.pending.is_empty() && x.play_area.is_empty())
    }

    pub fn process_input_leave(
        &mut self,
        input: GamePos,
        pointer: u64,
    ) -> Option<(NoteHitResult, PlayingNoteType)> {
        self.pointers.remove(&pointer);

        use rayon::iter::*;
        self.long_notes
            .par_iter_mut()
            .flat_map(|x| x.play_area.par_iter_mut())
            .filter(|x| x.is_x_in_range(input.x))
            .for_each(|playing_note| {
                playing_note.holding.remove(&pointer);
                if playing_note.holding.is_empty() {
                    if let Some(start_result) = playing_note.start_result {
                        for (p, input) in &self.pointers {
                            if playing_note.is_x_in_range(input.x) {
                                playing_note.holding.insert(*p);
                            }
                        }

                        if playing_note.holding.is_empty() && !self.auto_play {
                            let cur_result = self
                                .judge
                                .get_result(input.time, playing_note.note.end_time);
                            if cur_result.grade != NoteResult::Perfect {
                                playing_note.start_result =
                                    Some(NoteHitResult::new(NoteResult::Miss, start_result.delta));
                            }
                        }
                    }
                }
            });
        None
    }
}

macro_rules! impl_from_note {
    ($ty: ty, $tk: ident) => {
        impl<'a> From<&'a mut PlayingNote<$ty>> for PlayingNoteType<'a> {
            fn from(value: &'a mut PlayingNote<$ty>) -> Self {
                Self::$tk(value)
            }
        }
    };
}
impl_from_note!(NormalNote, Normal);
impl_from_note!(LongNote, Long);
