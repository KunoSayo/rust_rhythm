use crate::game::beatmap::play::Gaming;
use crate::game::OffsetType;

pub struct HitSummary {
    /// Summary  [-305, -295] ... (-15, -5] (-5, 5) [5, 15) ... [295, 305]
    pub delay_count: Vec<u32>,
    pub mx: u32,
}

pub struct BeatmapPlayResult {
    pub score: u32,
    pub hit_summary: HitSummary,
}

impl HitSummary {
    #[inline]
    pub const fn start_offset() -> OffsetType {
        -305
    }
    #[inline]
    pub fn bottom_numbers() -> impl Iterator<Item = OffsetType> {
        (0..).map(|i| i * 10 + Self::start_offset()).take_while(|x| *x <= 305)
    }
}

impl BeatmapPlayResult {
    pub fn from_game(game: &Gaming) -> Self {
        let score = &game.score_counter;
        let mut delay_count = vec![];
        let mut start_time_map_idx = vec![];
        for left in HitSummary::bottom_numbers() {
            delay_count.push(0);
            if left <= -5 {
                start_time_map_idx.push(left + 1);
            } else {
                start_time_map_idx.push(left);
            }
        }
        
        // we pop the last, for we consider range
        delay_count.pop();
        start_time_map_idx.pop();
        debug_assert!(!delay_count.is_empty());
        let mut mx = 1;
        for x in score.get_deltas() {
            match start_time_map_idx.binary_search(x) {
                Ok(idx) => {
                    delay_count[idx] += 1;
                }
                Err(idx) => {
                    delay_count[idx.saturating_sub(1)] += 1;
                }
            }
        }
        mx = mx.max(*delay_count.iter().max().unwrap_or(&0));
        Self {
            score: score.get_score(),
            hit_summary: HitSummary { delay_count, mx },
        }
    }
}
