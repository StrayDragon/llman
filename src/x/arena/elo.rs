use std::collections::HashMap;

pub const DEFAULT_K: f64 = 32.0;
pub const DEFAULT_INITIAL_RATING: f64 = 1500.0;

pub fn expected_score(rating_a: f64, rating_b: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf((rating_b - rating_a) / 400.0))
}

pub fn update(rating_a: f64, rating_b: f64, score_a: f64, k: f64) -> (f64, f64) {
    let exp_a = expected_score(rating_a, rating_b);
    let exp_b = 1.0 - exp_a;
    let score_b = 1.0 - score_a;
    (
        rating_a + k * (score_a - exp_a),
        rating_b + k * (score_b - exp_b),
    )
}

pub fn ensure_rating(map: &mut HashMap<String, f64>, id: &str) {
    map.entry(id.to_string()).or_insert(DEFAULT_INITIAL_RATING);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elo_update_changes_ratings_on_win() {
        let (a2, b2) = update(1500.0, 1500.0, 1.0, DEFAULT_K);
        assert!(a2 > 1500.0);
        assert!(b2 < 1500.0);
    }

    #[test]
    fn elo_update_keeps_sum_close() {
        let (a2, b2) = update(1500.0, 1600.0, 1.0, DEFAULT_K);
        // Elo is not strictly sum-preserving with rounding, but should be close here.
        let before = 1500.0 + 1600.0;
        let after = a2 + b2;
        assert!((before - after).abs() < 1e-9);
    }
}
