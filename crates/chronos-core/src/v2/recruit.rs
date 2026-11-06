//! Deterministic recruitment (§9.3, §23.2, §27.5): integer weights and a draw without
//! replacement by cumulative sums in ID order, each with an indexed sub-counter.
use super::ids::random_semantics_v2::uniform_below;
use super::model::{AlignmentAxes, HistoryError};
use serde::Serialize;

pub const RECRUIT_KEY: &str = "social/recruit/v1";

/// Scores of §23.2, each from 0 to 1,000.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Fits {
    pub skill: u32,
    pub activity: u32,
    pub status: u32,
    pub distance: u32,
    pub kin: u32,
    pub network: u32,
}
pub fn distance_fit(scale_days: u64, travel_days: u64) -> u32 {
    (1_000 * scale_days / (scale_days + travel_days).max(1)) as u32
}
/// Multiplicative factor of §27.5.
pub fn alignment_fit(person: AlignmentAxes, group: AlignmentAxes) -> u32 {
    let gap = (person.morality_bps() - group.morality_bps()).unsigned_abs()
        + (person.lawfulness_bps() - group.lawfulness_bps()).unsigned_abs();
    1_000u32.saturating_sub(gap / 20).max(100)
}
/// `weight_final` of §23.2: the alignment factor comes after the sum.
pub fn recruitment_weight(fits: &Fits, kin_bonus: u32, alignment: u32) -> u64 {
    let weight = 1_000
        + 3 * u64::from(fits.skill)
        + 2 * u64::from(fits.activity)
        + u64::from(fits.status)
        + u64::from(fits.distance)
        + u64::from(kin_bonus) * u64::from(fits.kin) / 1_000
        + u64::from(fits.network);
    weight * u64::from(alignment) / 1_000
}
/// One selection, with what a proof publishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Draw {
    pub index: usize,
    pub draw_ordinal: u32,
    pub value: u64,
    pub total_weight: u64,
    pub weight: u64,
}
/// Draws without replacement (§9.3). Keys hold `(request_id, effective_year,
/// vacancy_ordinal, draw_ordinal)`; `weights` follow ID order.
pub fn draw_without_replacement(
    seed: u64,
    request_id: &str,
    year: i32,
    vacancy_ordinal: u32,
    weights: &mut [u64],
    count: usize,
) -> Result<Vec<Draw>, HistoryError> {
    let year = year.to_string();
    let vacancy = vacancy_ordinal.to_string();
    let parts = [RECRUIT_KEY, request_id, year.as_str(), vacancy.as_str()];
    let mut total: u64 = weights.iter().sum();
    let mut draws = Vec::with_capacity(count);
    for draw_ordinal in 0..count as u32 {
        if total == 0 {
            break;
        }
        let value = uniform_below(seed, &parts, u64::from(draw_ordinal), total)?;
        let mut cursor = value;
        let index = weights
            .iter()
            .position(|w| {
                let hit = cursor < *w;
                if !hit {
                    cursor -= *w;
                }
                hit
            })
            .expect("value below the total");
        draws.push(Draw {
            index,
            draw_ordinal,
            value,
            total_weight: total,
            weight: weights[index],
        });
        total -= weights[index];
        weights[index] = 0;
    }
    Ok(draws)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alignment_fit_is_multiplicative_and_floored() {
        let saint = AlignmentAxes::new(9_000, 9_000).unwrap();
        let mafia = AlignmentAxes::new(-8_000, -6_000).unwrap();
        assert_eq!(alignment_fit(saint, saint), 1_000);
        assert_eq!(alignment_fit(saint, mafia), 100);
        let fits = Fits {
            skill: 1_000,
            ..Fits::default()
        };
        assert_eq!(recruitment_weight(&fits, 500, 100), 400);
    }
    #[test]
    fn draw_never_repeats_and_stops_when_empty() {
        let mut weights = [5, 0, 7];
        let draws = draw_without_replacement(1, "r", 1300, 0, &mut weights, 5).unwrap();
        let mut picked: Vec<_> = draws.iter().map(|d| d.index).collect();
        picked.sort();
        assert_eq!(picked, [0, 2]);
    }
}
