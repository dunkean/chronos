//! Diffuse influence of groups in space and time (§11). A reading of a packet, never a
//! fact source: it draws nothing, and jurisdiction stays an explicit relation.
use super::model::{ErrorCode, HistoryError};
use super::region::RegionPlan;
use super::social::{members_at, SocialPacket};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InfluenceContribution {
    pub group_id: String,
    pub source_site_id: String,
    pub travel_days: u64,
    pub value: u64,
    /// True when the group no longer exists and only its memory acts.
    pub residual: bool,
}
/// Sources acting on a site in a channel at a date, by decreasing value then ID, and
/// capped per channel. Strength is the realized membership, attenuated by travel time
/// and, after the group's end, by the channel's persistence. Travel also sets the first
/// year an influence can be seen, rounded up to a whole year.
pub fn influence_at(
    region: &RegionPlan,
    packet: &SocialPacket,
    site_id: &str,
    channel: &str,
    at_year: i32,
) -> Result<Vec<InfluenceContribution>, HistoryError> {
    let inputs = region.social.as_ref();
    let profile = inputs
        .catalogue
        .engine_defaults
        .influence_profiles
        .get(channel)
        .ok_or_else(|| HistoryError {
            owner: Some(channel.into()),
            at_year: Some(at_year),
            ..HistoryError::new(ErrorCode::InvalidCanonicalValue, "social.unknown_channel")
        })?;
    let days = inputs.atlas.travel_days_from(site_id, at_year);
    let budget = profile.travel_budget_days;
    let mut contributions = Vec::new();
    for (index, group) in packet.groups.iter().enumerate() {
        let Some(travel_days) = days.get(&group.site_id).copied().filter(|d| *d <= budget) else {
            continue;
        };
        let existence = group.entity.existence;
        let delay = travel_days.div_ceil(365) as i32;
        if at_year < existence.start_year + delay {
            continue;
        }
        let residual = at_year >= existence.end_year;
        let years_gone = at_year - existence.end_year;
        if residual && years_gone >= profile.persistence_years {
            continue;
        }
        let seen_at = if residual {
            existence.end_year - 1
        } else {
            at_year
        };
        let strength = members_at(packet, index as u32, seen_at).len() as u64 * 10_000;
        let attenuated = strength * (budget + 1 - travel_days) / (budget + 1);
        let decay = if residual {
            10_000u64.saturating_sub(u64::from(profile.decay_bps_per_year) * years_gone as u64)
        } else {
            10_000
        };
        let value = attenuated * decay / 10_000;
        if value > 0 {
            contributions.push(InfluenceContribution {
                group_id: group.entity.id.clone(),
                source_site_id: group.site_id.clone(),
                travel_days,
                value,
                residual,
            });
        }
    }
    contributions.sort_by(|a, b| b.value.cmp(&a.value).then(a.group_id.cmp(&b.group_id)));
    contributions.truncate(profile.max_sources);
    Ok(contributions)
}
