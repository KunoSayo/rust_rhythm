use crate::game::beatmap::play::{Gaming, ScoreCounter};

pub struct HitSummary {
    /// Summary  [-305, -295) ... [-15, -6] [-5, 5] [6, 15] ... (295, 305]  
    delay_count: Vec<u32>
} 

pub struct BeatmapPlayResult {
    pub score: u32,
    hit_summary: HitSummary
}

impl BeatmapPlayResult {
    pub fn from_game(game: &Gaming) -> Self {
        let score = &game.score_counter;
        let mut delay_count = vec![];
        let mut left = -305;
        let mut start_time_map_idx = vec![];
        while left <= 295 {
            delay_count.push(0);
            if left <= -5 {
                start_time_map_idx.push(left);
            } else {
                start_time_map_idx.push(left + 1);
            }
            left += 10;
        }
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
        Self {
            score: score.get_score(),
            hit_summary: HitSummary {delay_count},
        }
    }
}