//! Procedural social attributes of the candidate table (§8.4, §25.5, §27.5). They
//! live in the pure core, after the provider: nothing here reaches the worker, and
//! an attribute the simulated core already established is never rewritten.
use super::catalog::CapacityBand;
use super::ids::random_semantics_v2::uniform_below;
use super::ids::{DomainKey, SemanticAddress};
use super::model::{AlignmentAxes, HistoryError};
use super::population::{CandidateTable, SlotSource};
use super::social::{SocialDiagnostic, SocialInputs};
use serde::Serialize;
use std::collections::BTreeMap;

pub const ALIGNMENT_SUBSTREAM: &str = "alignment/v1";
const ATTRIBUTES: &str = "social/attributes/v1";
const BPS: u64 = 10_000;
const AXIS_DRAW_BOUND: i32 = 1_500;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonAttributes {
    pub person_id: String,
    pub status_id: String,
    pub status_rank: u32,
    pub activity_id: String,
    /// Stable personal draw of §25.5; a civic charge does not require it.
    pub workforce: bool,
    pub culture_id: String,
    /// Family affinity latent of §8.4, in bps, shared by the family key.
    pub family_latent_bps: u32,
    pub axes: AlignmentAxes,
    /// `skill_initial` per activity (§25.5).
    pub skill_initial: BTreeMap<String, u32>,
}

/// Semantic text of a provider key without the domain it repeats: an address field
/// never embeds an encoded ID (§4.1).
pub fn local_key(text: &str, domain_id: &str) -> String {
    text.replacen(&format!("\"{domain_id}\","), "", 1)
}
/// Sum of three uniform integers in `[-bound, bound]` (§27.5).
pub(crate) fn three_draws(seed: u64, parts: &[&str], bound: i32) -> Result<i32, HistoryError> {
    if bound <= 0 {
        return Ok(0);
    }
    let width = 2 * bound as u64 + 1;
    (0..3).try_fold(0, |sum, i| {
        Ok(sum + uniform_below(seed, parts, i, width)? as i32 - bound)
    })
}
pub(crate) fn clamp_axes(morality: i32, lawfulness: i32) -> Result<AlignmentAxes, HistoryError> {
    let bound = AlignmentAxes::MAX_ABS_BPS;
    AlignmentAxes::new(
        morality.clamp(-bound, bound),
        lawfulness.clamp(-bound, bound),
    )
}
/// Selector of §9.4: a Bernoulli picks either the shared family uniform or the
/// individual one, never their mean, so the margin stays uniform.
fn latent_uniform(
    seed: u64,
    topic: &str,
    family: &str,
    person: &str,
    inherit_bps: u32,
    upper: u64,
) -> Result<u64, HistoryError> {
    let selector = uniform_below(seed, &[ATTRIBUTES, topic, "selector", person], 0, BPS)?;
    if selector < u64::from(inherit_bps) {
        uniform_below(seed, &[ATTRIBUTES, topic, "family", family], 0, upper)
    } else {
        uniform_below(seed, &[ATTRIBUTES, topic, "person", person], 0, upper)
    }
}
/// Entry reached by a uniform in `[0, total)`, over weights in key order.
fn pick<'a, K>(weights: impl Iterator<Item = (&'a K, u64)>, mut value: u64) -> Option<&'a K> {
    for (key, weight) in weights {
        if value < weight {
            return Some(key);
        }
        value -= weight;
    }
    None
}
fn weights_u32(map: &BTreeMap<String, u32>) -> impl Iterator<Item = (&String, u64)> {
    map.iter().map(|(k, w)| (k, u64::from(*w)))
}
pub(crate) fn capacity_bps(bands: &[CapacityBand], age: i32) -> u16 {
    bands
        .iter()
        .find(|b| i64::from(b.min_age) <= i64::from(age) && i64::from(age) <= i64::from(b.max_age))
        .map_or(0, |b| b.capacity_bps)
}

pub(crate) struct AttributeContext<'a> {
    pub inputs: &'a SocialInputs,
    pub table: &'a CandidateTable,
    pub domain_id: &'a str,
    pub domain: &'a DomainKey,
    pub seed: u64,
    /// Target status weights of the domain's social profile, by class ID.
    pub status_weights: &'a BTreeMap<String, u64>,
    /// First residence of each slot, with the year it was first observed there.
    pub first_site: &'a [Option<(&'a str, i32)>],
}
/// Residues of §8.4: target minus core per class, renormalized on the background.
/// All-zero residues fall back on the base weights, with a visible diagnostic.
fn background_status_weights(
    c: &AttributeContext,
    ranks: &[(String, u32)],
    diagnostics: &mut Vec<SocialDiagnostic>,
) -> BTreeMap<u32, u64> {
    let total_weight: u64 = c.status_weights.values().sum();
    let weight_of = |id: &str| c.status_weights.get(id).copied().unwrap_or(0);
    let mut core: BTreeMap<u32, u64> = BTreeMap::new();
    for attributes in c.table.core_attributes.iter().flatten() {
        *core.entry(attributes.status_rank).or_default() += 1;
    }
    let slots = c.table.len() as u64;
    let residue: BTreeMap<u32, u64> = ranks
        .iter()
        .map(|(id, rank)| {
            let target = slots * weight_of(id) / total_weight.max(1);
            let realized = core.get(rank).copied().unwrap_or(0);
            (*rank, target.saturating_sub(realized))
        })
        .collect();
    if residue.values().any(|w| *w > 0) {
        return residue;
    }
    diagnostics.push(SocialDiagnostic {
        code: "social.status_residue_empty".into(),
        subject: c.domain_id.into(),
        year: None,
    });
    ranks
        .iter()
        .map(|(id, rank)| (*rank, weight_of(id)))
        .collect()
}
/// Attributes of every slot. Ancestry is never read: conduct and status come from
/// culture, family and person keys only (§27.1).
pub(crate) fn person_attributes(
    c: &AttributeContext,
    diagnostics: &mut Vec<SocialDiagnostic>,
) -> Result<Vec<PersonAttributes>, HistoryError> {
    let inputs = c.inputs;
    let defaults = &inputs.catalogue.engine_defaults;
    let seed = c.seed;
    let mut ranks: Vec<(String, u32)> = inputs
        .social_classes
        .iter()
        .map(|s| (s.id.clone(), s.provider_rank as u32))
        .collect();
    ranks.sort_by_key(|(_, rank)| *rank);
    let background = background_status_weights(c, &ranks, diagnostics);
    let background_total: u64 = background.values().sum();
    let status_by_rank: BTreeMap<u32, &str> = ranks
        .iter()
        .map(|(id, rank)| (*rank, id.as_str()))
        .collect();
    let activities = &inputs.catalogue.required_activity_coverage;
    let mut families: BTreeMap<&str, (i32, i32, u32)> = BTreeMap::new();
    let mut people = Vec::with_capacity(c.table.len());
    for slot in 0..c.table.len() {
        let origin = local_key(&c.table.origin_id[slot], c.domain_id);
        let family_text = c.table.family_key[slot].as_str();
        let family = local_key(family_text, c.domain_id);
        let episode = c.first_site[slot].and_then(|(site, year)| {
            let anchor = inputs
                .atlas
                .settlements
                .iter()
                .find(|s| s.site_id == site)?;
            anchor.episode_at(year.max(inputs.timeline.start_year))
        });
        // A family shares its culture: a stable choice among the site's weights.
        let culture_id = match episode {
            Some(e) => {
                let total: u64 = e.culture_weights.values().map(|w| u64::from(*w)).sum();
                let parts = [ATTRIBUTES, "culture", family.as_str()];
                let value = uniform_below(seed, &parts, 0, total.max(1))?;
                pick(weights_u32(&e.culture_weights), value)
                    .cloned()
                    .unwrap_or_default()
            }
            None => String::new(),
        };
        let core = c.table.core_attributes[slot]
            .as_ref()
            .filter(|_| c.table.source[slot] == SlotSource::Simulated);
        let status_rank = match core {
            Some(core) => core.status_rank,
            None => {
                let value = latent_uniform(
                    seed,
                    "status",
                    &family,
                    &origin,
                    defaults.status_inheritance_bps,
                    background_total.max(1),
                )?;
                pick(background.iter().map(|(k, w)| (k, *w)), value)
                    .copied()
                    .unwrap_or(0)
            }
        };
        let activity_id = match (core, episode) {
            (Some(core), _) => core.activity_id.clone(),
            (None, Some(e)) => {
                let total: u64 = e.activity_weights.values().map(|w| u64::from(*w)).sum();
                let value = latent_uniform(
                    seed,
                    "activity",
                    &family,
                    &origin,
                    defaults.activity_inheritance_bps,
                    total.max(1),
                )?;
                pick(weights_u32(&e.activity_weights), value)
                    .cloned()
                    .unwrap_or_default()
            }
            (None, None) => String::new(),
        };
        let workforce = uniform_below(seed, &[ATTRIBUTES, "workforce", &origin], 0, BPS)?
            < defaults.adult_workforce_participation_bps;
        let (family_morality, family_lawfulness, family_latent_bps) =
            match families.get(family_text) {
                Some(known) => *known,
                None => {
                    let axis = |name| {
                        let parts = [ALIGNMENT_SUBSTREAM, "family", name, family.as_str()];
                        three_draws(seed, &parts, AXIS_DRAW_BOUND)
                    };
                    let latent = uniform_below(seed, &[ATTRIBUTES, "latent", &family], 0, BPS)?;
                    let drawn = (axis("morality")?, axis("lawfulness")?, latent as u32);
                    families.insert(family_text, drawn);
                    drawn
                }
            };
        let base = inputs
            .catalogue
            .binding(&culture_id)
            .ok()
            .and_then(|b| {
                inputs
                    .catalogue
                    .family_value_profiles
                    .get(&b.history_family_id)
            })
            .map_or((0, 0), |v| {
                (
                    v.alignment_base.morality_bps,
                    v.alignment_base.lawfulness_bps,
                )
            });
        let individual = |name| {
            let parts = [ALIGNMENT_SUBSTREAM, "person", name, origin.as_str()];
            three_draws(seed, &parts, AXIS_DRAW_BOUND)
        };
        let axes = clamp_axes(
            base.0 + family_morality + individual("morality")?,
            base.1 + family_lawfulness + individual("lawfulness")?,
        )?;
        let upper = u64::from(defaults.skill_initial_max) + 1;
        let skill_initial = activities
            .iter()
            .map(|a| {
                let value = uniform_below(seed, &[ATTRIBUTES, "skill", &origin, a], 0, upper)?;
                Ok((a.clone(), value as u32))
            })
            .collect::<Result<_, HistoryError>>()?;
        let person = SemanticAddress::person(c.domain, &inputs.provider_algorithm, &origin)?;
        people.push(PersonAttributes {
            person_id: person.encode(),
            status_id: status_by_rank
                .get(&status_rank)
                .copied()
                .unwrap_or_default()
                .into(),
            status_rank,
            activity_id,
            workforce,
            culture_id,
            family_latent_bps,
            axes,
            skill_initial,
        });
    }
    Ok(people)
}
