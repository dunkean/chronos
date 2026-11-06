//! Exposure resolution over the owner's complete canonical catalogue.
use super::atlas::{ContactEnd, HistoryAtlas, SettlementAnchor};
use super::canonical::canonical_value_bytes;
use super::catalog::ArchetypeDefinition;
use super::general::{
    blocks, date_in_block, gate, largest_remainder, principal_culture, EventLedger, GeneralCanon,
    NewEvent, Polity,
};
use super::ids::random_semantics_v2::{draw, uniform_below};
use super::ids::{decade_block, AddressKind, Owner, OwnerScope, SemanticAddress};
use super::model::{
    string_enum, Agency, EntityNature, ErrorCode, EventPhase, HistoryError, HistoryEvent,
    LegalStanding, LegalStatus, ObjectRef, SocialEntity, TimeSpan, Visibility,
};
use super::population::{
    BoundaryPort, PortMode, PortRateBasis, PortRateEpisode, PortUnit, ShockEffects,
};
use super::validate::{
    causal_ranks, validate_legal_standings, LEGAL_CHANGE_EVENT_KINDS,
    SUBJECT_DISSOLUTION_EVENT_KIND, SUBJECT_FOUNDATION_EVENT_KIND,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EXACT_CANDIDATES: usize = 100_000;
string_enum!(ImpactTarget {
    Sites,
    People,
    Cohorts,
    Organizations,
    Assets
});
string_enum!(RoundingRule { Floor, Ceil });
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImpactMode {
    Probability {
        probability_bps: u16,
    },
    ExactCount {
        count: u64,
    },
    ExactFraction {
        numerator: u64,
        denominator: u64,
        rounding: RoundingRule,
    },
    FixedTarget {
        target_ref: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "ids",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetSelector {
    All,
    Ids(BTreeSet<String>),
}
string_enum!(ExposureModel {
    IndependentTargetV1
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactContract {
    pub id: String,
    pub parent_event_id: String,
    pub owner_scope: OwnerScope,
    pub valid: TimeSpan,
    pub target: ImpactTarget,
    pub selector: TargetSelector,
    pub eligibility_time: i32,
    #[serde(flatten)]
    pub mode: ImpactMode,
    pub exposure_model: ExposureModel,
    pub severity_profile: String,
    pub timing_profile: String,
    pub channel: String,
    pub cause: String,
}
impl ImpactContract {
    /// `id` decodes to an impact address, child of `parent_event_id` and carried by
    /// `owner_scope`. Returns this address, parent of the effects.
    pub fn validate(&self) -> Result<SemanticAddress, HistoryError> {
        let fail = |message: &str| HistoryError {
            owner: Some(self.owner_scope.to_string()),
            at_year: Some(self.eligibility_time),
            ..HistoryError::new(ErrorCode::InvalidImpactContract, message)
        };
        let impact = SemanticAddress::decode(&self.id)
            .ok()
            .filter(|a| a.kind() == AddressKind::Impact)
            .ok_or_else(|| fail("impact.id_not_impact"))?;
        if impact.parent_event().map(|e| e.encode()).as_ref() != Some(&self.parent_event_id) {
            return Err(fail("impact.foreign_parent_event"));
        }
        if impact.owner().as_ref().map(|o| o.scope()) != Some(&self.owner_scope) {
            return Err(fail("entity.owner_scope_differs"));
        }
        Ok(impact)
    }
}
/// Complete catalogue of an owner. It has no public field and no `Deserialize`
/// and is built only from an authoritative core source, which itself enumerates
/// the candidates: no list is received from the caller. S0 has only the
/// atlas; S2 will add other sources in the same way.
///
/// ```compile_fail
/// use chronos_core::v2::region::CompleteCandidateCatalogue;
/// let forged: CompleteCandidateCatalogue = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteCandidateCatalogue {
    owner: Owner,
    target: ImpactTarget,
    candidates: BTreeMap<String, TimeSpan>,
}
impl CompleteCandidateCatalogue {
    /// Owner's sites in the validated atlas. A domain is not known to the
    /// atlas: its partition belongs to S2.
    pub fn from_atlas(atlas: &HistoryAtlas, owner: &Owner) -> Result<Self, HistoryError> {
        atlas.validate()?;
        let fail = |code, message: &str| HistoryError {
            owner: Some(owner.scope().to_string()),
            at_year: Some(atlas.timeline.start_year),
            ..HistoryError::new(code, message)
        };
        let within = |cell_id: &str, site_id: &str| match owner.scope() {
            OwnerScope::Cell { cell_id: root } => atlas.cell_is_within(cell_id, root),
            OwnerScope::Site { site_id: only } => site_id == only,
            _ => true,
        };
        let known = match owner.scope() {
            OwnerScope::World => true,
            OwnerScope::Cell { cell_id } => atlas.cells.iter().any(|c| &c.cell_id == cell_id),
            OwnerScope::Site { site_id } => atlas.settlements.iter().any(|s| &s.site_id == site_id),
            OwnerScope::Domain { .. } => {
                return Err(fail(
                    ErrorCode::UnsupportedCapability,
                    "target.domain_catalogue_partition",
                ));
            }
        };
        if !known {
            return Err(fail(ErrorCode::InvalidAtlas, "target.owner_missing"));
        }
        Ok(Self {
            owner: owner.clone(),
            target: ImpactTarget::Sites,
            candidates: atlas
                .settlements
                .iter()
                .filter(|s| within(&s.cell_id, &s.site_id))
                .map(|s| (s.site_id.clone(), atlas.timeline))
                .collect(),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetImpact {
    pub target_id: String,
    pub event_id: String,
    /// Effect specific to this target, child of `event_id`.
    pub effect_id: String,
    /// Random intensity word, independent of the exposure; the business profile
    /// that will interpret it belongs to S2.
    pub intensity_draw: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactResolution {
    pub contract_id: String,
    pub targets: Vec<TargetImpact>,
}
impl ImpactResolution {
    pub fn for_target(&self, id: &str) -> Option<&TargetImpact> {
        self.targets.iter().find(|t| t.target_id == id)
    }
    /// Projection only: no draw and no new quota resolution.
    pub fn project<'a>(&'a self, ids: &BTreeSet<String>) -> Vec<&'a TargetImpact> {
        self.targets
            .iter()
            .filter(|t| ids.contains(&t.target_id))
            .collect()
    }
}
/// Resolves exposure over the whole catalogue. In exact mode, K is computed over
/// the candidates eligible at `eligibility_time` and retained by the selector,
/// never over the raw catalogue: a fraction applies to what can be affected.
pub fn resolve_impact(
    seed: u64,
    contract: &ImpactContract,
    catalogue: &CompleteCandidateCatalogue,
) -> Result<ImpactResolution, HistoryError> {
    let located = |e: HistoryError| HistoryError {
        owner: Some(contract.owner_scope.to_string()),
        at_year: Some(contract.eligibility_time),
        ..e
    };
    let fail = |code, s: &str| located(HistoryError::new(code, s));
    contract.valid.validate(false).map_err(located)?;
    let impact = contract.validate()?;
    if !matches!(contract.mode, ImpactMode::Probability { .. })
        && matches!(
            contract.target,
            ImpactTarget::People | ImpactTarget::Cohorts
        )
    {
        return Err(fail(
            ErrorCode::UnsupportedCapability,
            "target.exact_demographic_reserved",
        ));
    }
    // Contract and catalogue share canon and owner; effects inherit them
    // through their address.
    if !contract.valid.contains(contract.eligibility_time)
        || impact.owner().as_ref() != Some(&catalogue.owner)
        || contract.target != catalogue.target
        || [
            &contract.severity_profile,
            &contract.timing_profile,
            &contract.channel,
            &contract.cause,
        ]
        .iter()
        .any(|id| !super::atlas::valid_id(id))
    {
        return Err(fail(
            ErrorCode::InvalidImpactContract,
            "target.contract_or_catalogue_incompatible",
        ));
    }
    if !matches!(contract.mode, ImpactMode::Probability { .. })
        && catalogue.candidates.len() > MAX_EXACT_CANDIDATES
    {
        return Err(fail(
            ErrorCode::BudgetExceeded,
            "target.catalogue_too_large",
        ));
    }
    if let TargetSelector::Ids(ids) = &contract.selector {
        if ids.iter().any(|id| !catalogue.candidates.contains_key(id)) {
            return Err(fail(
                ErrorCode::InvalidImpactContract,
                "target.selector_outside_catalogue",
            ));
        }
    }
    let candidates: Vec<_> = catalogue
        .candidates
        .iter()
        .filter(|(id, span)| {
            span.contains(contract.eligibility_time)
                && match &contract.selector {
                    TargetSelector::All => true,
                    TargetSelector::Ids(ids) => ids.contains(*id),
                }
        })
        .map(|(id, _)| id)
        .collect();
    let mut selected = Vec::new();
    if let ImpactMode::Probability { probability_bps } = contract.mode {
        if probability_bps > 10_000 {
            return Err(fail(
                ErrorCode::InvalidImpactContract,
                "target.probability_out_of_bps",
            ));
        }
        for id in candidates {
            if uniform_below(seed, &[&contract.id, id, "exposure"], 0, 10_000)?
                < u64::from(probability_bps)
            {
                selected.push(id);
            }
        }
    } else {
        let k = match &contract.mode {
            ImpactMode::ExactCount { count } => u128::from(*count),
            ImpactMode::ExactFraction {
                numerator,
                denominator,
                rounding,
            } => {
                if *denominator == 0 || numerator > denominator {
                    return Err(fail(
                        ErrorCode::InvalidImpactContract,
                        "target.fraction_invalid",
                    ));
                }
                let product = (candidates.len() as u128) * u128::from(*numerator);
                match rounding {
                    RoundingRule::Floor => product / u128::from(*denominator),
                    RoundingRule::Ceil => product.div_ceil(u128::from(*denominator)),
                }
            }
            ImpactMode::FixedTarget { target_ref } => {
                if !candidates.contains(&target_ref) {
                    return Err(fail(
                        ErrorCode::ExactImpactInfeasible,
                        "target.fixed_target_ineligible",
                    ));
                }
                selected.push(target_ref);
                0
            }
            ImpactMode::Probability { .. } => unreachable!(),
        };
        if k > candidates.len() as u128 {
            return Err(fail(
                ErrorCode::ExactImpactInfeasible,
                "target.k_exceeds_eligible",
            ));
        }
        if !matches!(contract.mode, ImpactMode::FixedTarget { .. }) {
            let mut ranked: Vec<_> = candidates
                .into_iter()
                .map(|id| (draw(seed, &[&contract.id, id, "priority"], 0, 0), id))
                .collect();
            ranked.sort();
            selected.extend(ranked.into_iter().take(k as usize).map(|(_, id)| id));
        }
    }
    selected.sort();
    let targets = selected
        .into_iter()
        .map(|id| {
            Ok(TargetImpact {
                target_id: id.clone(),
                event_id: contract.parent_event_id.clone(),
                effect_id: SemanticAddress::site(catalogue.owner.world(), id)
                    .and_then(|target| SemanticAddress::effect(&impact, &target))
                    .map_err(located)?
                    .encode(),
                intensity_draw: draw(seed, &[&contract.id, id, "intensity"], 0, 0),
            })
        })
        .collect::<Result<_, HistoryError>>()?;
    Ok(ImpactResolution {
        contract_id: contract.id.clone(),
        targets,
    })
}

/// Annual integrated effects and durations of a severity profile (§7.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeverityProfile {
    pub id: &'static str,
    pub effects: ShockEffects,
    pub duration_years: i32,
    pub recovery_years: u64,
    /// `ruin` also damages the site; it does not mean everyone dies.
    pub material_damage: bool,
}
/// Initial table of §7.5.
pub fn severity_profile(id: &str) -> Option<SeverityProfile> {
    let (id, mortality, fertility, capacity, duration_years, recovery_years) = match id {
        "plague.low" => ("plague.low", 300, 900_000, None, 2, 3),
        "plague.medium" => ("plague.medium", 800, 700_000, None, 3, 8),
        "plague.high" => ("plague.high", 1_500, 500_000, None, 3, 12),
        "war.ruin" => ("war.ruin", 100, 700_000, Some(400_000), 1, 10),
        "harvest.bad" => ("harvest.bad", 0, 850_000, Some(700_000), 2, 5),
        _ => return None,
    };
    Some(SeverityProfile {
        id,
        effects: ShockEffects {
            additional_mortality_bps: (mortality > 0).then_some(mortality),
            mortality_multiplier_ppm: None,
            fertility_multiplier_ppm: Some(fertility),
            migration_multiplier_ppm: None,
            capacity_multiplier_ppm: capacity,
        },
        duration_years,
        recovery_years,
        material_damage: id == "war.ruin",
    })
}
/// Plague tier read from the intensity word of its separate key (§7.2). Quantile
/// in bps: half low, 35 % medium, 15 % high.
fn plague_tier(intensity_draw: u64) -> &'static str {
    match (u128::from(intensity_draw) * 10_000) >> 64 {
        0..=4_999 => "plague.low",
        5_000..=8_499 => "plague.medium",
        _ => "plague.high",
    }
}
/// Exposure of one site to one contract, compiled into a local shock. The site
/// is already exposed: the provider never draws a second time (§24.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LocalShock {
    pub effect_id: String,
    pub episode_id: String,
    pub source_event_id: String,
    pub site_id: String,
    pub severity_profile_id: String,
    pub valid: TimeSpan,
    pub recovery_years: u64,
    pub effects: ShockEffects,
    pub material_damage: bool,
}
/// Planned household move after a ruin; `move_households` in the plan (§7.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Displacement {
    pub event_id: String,
    pub op_id: String,
    pub origin_site_id: String,
    pub destination_site_id: Option<String>,
    pub destination_port_id: Option<String>,
    pub year: i32,
    pub target_households: u64,
}
/// Outside context of a domain (§12): detailed ports, then an aggregate summary.
/// The halo never calls a neighbouring simulation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExternalContext {
    pub ports: Vec<BoundaryPort>,
    /// Routes leaving the domain, kept as context only.
    pub halo_edge_ids: Vec<String>,
    pub omitted_contacts: u64,
    pub omitted_population_expected: u64,
}
/// Regional collective. S2 creates own-stock and event-born groups only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegionGroup {
    pub entity: SocialEntity,
    pub site_id: String,
}
/// Regional plan (§3, §14.1): a pure function of the canon and the region ID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegionPlan {
    pub canon_id: String,
    pub region_id: String,
    /// Shock-free site markers, `year -> population`.
    pub site_baselines: BTreeMap<String, BTreeMap<i32, u64>>,
    /// Separate forecast (§7.4), never a marker source.
    pub population_expected_after_impacts: BTreeMap<String, BTreeMap<i32, u64>>,
    pub shocks: Vec<LocalShock>,
    pub displacements: Vec<Displacement>,
    pub contexts: BTreeMap<String, ExternalContext>,
    pub groups: Vec<RegionGroup>,
    pub events: Vec<HistoryEvent>,
    pub standings: Vec<LegalStanding>,
}
impl RegionPlan {
    /// Canonical bytes: `plan_region` has already sorted the lists.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, HistoryError> {
        canonical_value_bytes(self)
    }
}
fn region_sites<'a>(canon: &'a GeneralCanon, region: &str) -> Vec<&'a SettlementAnchor> {
    let mut sites: Vec<_> = canon
        .atlas
        .settlements
        .iter()
        .filter(|s| canon.atlas.region_of(s) == Some(region))
        .collect();
    sites.sort_by(|a, b| a.site_id.cmp(&b.site_id));
    sites
}
fn capacity_at(site: &SettlementAnchor, year: i32) -> u64 {
    site.episode_at(year).map_or(0, |e| e.capacity)
}
/// Site markers of §26.5. Explicit author series stay as given. Otherwise the
/// regional base is projected on each grid date, with weights equal to initial
/// population times potential capacity over initial capacity, quantized in ppm.
fn site_baselines(
    canon: &GeneralCanon,
    region: &str,
) -> Result<BTreeMap<String, BTreeMap<i32, u64>>, HistoryError> {
    let sites = region_sites(canon, region);
    let explicit = &canon.constraints.site_baselines;
    if sites.iter().any(|s| explicit.contains_key(&s.site_id)) {
        return Ok(sites
            .iter()
            .map(|s| (s.site_id.clone(), explicit[&s.site_id].clone()))
            .collect());
    }
    let start = canon.timeline.start_year;
    let mut series: BTreeMap<String, BTreeMap<i32, u64>> = BTreeMap::new();
    for (year, total) in &canon.baselines[region].grid {
        let weights: Vec<_> = sites
            .iter()
            .map(|s| {
                let potential = u128::from(capacity_at(s, *year)) * 1_000_000
                    / u128::from(capacity_at(s, start).max(1));
                (
                    s.site_id.as_str(),
                    u128::from(s.initial_population) * potential,
                )
            })
            .collect();
        for ((site, _), population) in weights.iter().zip(largest_remainder(*total, &weights)) {
            series
                .entry((*site).to_owned())
                .or_default()
                .insert(*year, population);
        }
    }
    Ok(series)
}
/// Linear interpolation between markers; it anticipates no crisis (§7.4).
fn baseline_at(series: &BTreeMap<i32, u64>, year: i32) -> u64 {
    let before = series.range(..=year).next_back();
    let after = series.range(year..).next();
    match (before, after) {
        (Some((y0, p0)), Some((y1, p1))) if y1 > y0 => {
            let (p0, p1) = (i128::from(*p0), i128::from(*p1));
            let span = i128::from(y1 - y0);
            (p0 + (p1 - p0) * i128::from(year - y0) / span) as u64
        }
        (Some((_, p)), _) | (None, Some((_, p))) => *p,
        (None, None) => 0,
    }
}
/// Resolves every regional contract on the complete catalogue and compiles the
/// exposures. A shock outside the world's period is dropped (§7.1).
fn compile_impacts(canon: &GeneralCanon, region: &str) -> Result<Vec<LocalShock>, HistoryError> {
    let owner = Owner::cell(&canon.canon_id, region);
    let catalogue = CompleteCandidateCatalogue::from_atlas(&canon.atlas, &owner)?;
    let seed = canon.world_seed;
    let mut shocks = Vec::new();
    for contract in canon
        .contracts
        .iter()
        .filter(|c| &c.owner_scope == owner.scope())
    {
        let resolution = resolve_impact(seed, contract, &catalogue)?;
        for target in &resolution.targets {
            let site = target.target_id.as_str();
            let keys = [contract.id.as_str(), site, "timing"];
            let frame = contract.valid;
            let (profile_id, earliest, latest) = if contract.cause == "war" {
                let Some(earliest) = military_arrival(canon, contract, site)? else {
                    continue;
                };
                ("war.ruin", earliest, frame.end_year - 1)
            } else {
                let id = plague_tier(target.intensity_draw);
                let duration = severity_profile(id).map_or(1, |p| p.duration_years);
                let latest = (frame.end_year - duration).max(frame.start_year);
                (id, frame.start_year, latest)
            };
            if earliest > latest {
                continue;
            }
            let profile = severity_profile(profile_id).ok_or_else(|| {
                HistoryError::new(ErrorCode::InvalidImpactContract, "impact.unknown_severity")
            })?;
            let start =
                earliest + uniform_below(seed, &keys, 0, (latest - earliest + 1) as u64)? as i32;
            let effect = SemanticAddress::decode(&target.effect_id)?;
            shocks.push(LocalShock {
                effect_id: target.effect_id.clone(),
                episode_id: SemanticAddress::episode(&effect, "episode-0")?.encode(),
                source_event_id: target.event_id.clone(),
                site_id: site.into(),
                severity_profile_id: profile.id.into(),
                valid: TimeSpan {
                    start_year: start,
                    end_year: (start + profile.duration_years).min(canon.timeline.end_year),
                },
                recovery_years: profile.recovery_years,
                effects: profile.effects,
                material_damage: profile.material_damage,
            });
        }
    }
    shocks.sort_by(|a, b| a.effect_id.cmp(&b.effect_id));
    Ok(shocks)
}
/// Earliest arrival on a site: `start + ceil(travel_days / 365)` from the seat of
/// the party that does not hold it (§7.5). `None` when no route reaches it.
fn military_arrival(
    canon: &GeneralCanon,
    contract: &ImpactContract,
    site: &str,
) -> Result<Option<i32>, HistoryError> {
    let start = contract.valid.start_year;
    let event = canon
        .events
        .iter()
        .find(|e| e.id == contract.parent_event_id)
        .ok_or_else(|| {
            HistoryError::new(
                ErrorCode::InvalidImpactContract,
                "impact.parent_event_missing",
            )
        })?;
    let holder = canon
        .controller_at(site, start)
        .map(|p| p.entity.id.as_str());
    let attacker = event.participants.iter().find_map(|party| match party {
        ObjectRef::Entity(id) if Some(id.as_str()) != holder => {
            canon.polities.iter().find(|p| &p.entity.id == id)
        }
        _ => None,
    });
    Ok(attacker.and_then(|polity| {
        let days = *canon
            .atlas
            .travel_days_from(&polity.seat_site_id, start)
            .get(site)?;
        Some(start + days.div_ceil(365) as i32)
    }))
}
/// Ports and halo of each domain of the region (§12). Contacts sort by travel
/// time then ID; beyond `max_halo_contacts` they survive as a summary only.
fn external_contexts(
    canon: &GeneralCanon,
    region: &str,
) -> Result<BTreeMap<String, ExternalContext>, HistoryError> {
    let atlas = &canon.atlas;
    let limit = canon.config.population.limits.max_halo_contacts as usize;
    let days_of = |edge: &Option<String>| {
        edge.as_ref()
            .and_then(|id| atlas.mobility_edges.iter().find(|e| &e.edge_id == id))
            .map_or(0, |e| e.travel_days)
    };
    let mut contexts = BTreeMap::new();
    for domain in canon.domains.iter().filter(|d| d.region_id == region) {
        let inside: BTreeSet<_> = domain.site_ids.iter().map(String::as_str).collect();
        let mut attached = Vec::new();
        for contact in &atlas.external_contacts {
            for (index, end) in contact.ends.iter().enumerate() {
                if let ContactEnd::Site(side) = end {
                    if inside.contains(side.site_id.as_str()) {
                        let days = days_of(&side.access_edge_id);
                        attached.push((
                            days,
                            &contact.contact_id,
                            &side.site_id,
                            index,
                            contact,
                            side,
                        ));
                    }
                }
            }
        }
        attached.sort_by_key(|a| (a.0, a.1, a.2));
        let mut ports = Vec::new();
        let (mut omitted_contacts, mut omitted_population_expected) = (0, 0);
        for (position, (_, _, _, index, contact, side)) in attached.into_iter().enumerate() {
            if position >= limit {
                omitted_contacts += 1;
                omitted_population_expected += side.population_expected;
                continue;
            }
            let rate_episodes = canon.contact_flows[&contact.contact_id]
                .iter()
                .map(|flow| {
                    // Both sides share one flow: one side's exit is the other's entry.
                    let (outgoing_rate_bps, incoming_rate_bps) = if index == 0 {
                        (flow.forward_rate_bps, flow.backward_rate_bps)
                    } else {
                        (flow.backward_rate_bps, flow.forward_rate_bps)
                    };
                    PortRateEpisode {
                        valid: flow.valid,
                        incoming_rate_bps,
                        outgoing_rate_bps,
                    }
                })
                .collect();
            ports.push(BoundaryPort {
                port_id: format!("port/{}/{}", contact.contact_id, side.site_id),
                contact_id: contact.contact_id.clone(),
                mode: PortMode::StatisticalReservoir,
                entry_site_id: side.site_id.clone(),
                external_context_id: side.external_context_id.clone(),
                external_profile_id: side.external_profile_id.clone(),
                population_expected: side.population_expected,
                valid: contact.valid,
                unit: PortUnit::Household,
                rate_basis: PortRateBasis::ResidentHouseholdsYearOpening,
                rate_period_years: 1,
                rate_episodes,
                bridge_plan_id: None,
            });
        }
        let halo_edge_ids = atlas
            .mobility_edges
            .iter()
            .filter(|e| {
                inside.contains(e.from_site_id.as_str()) != inside.contains(e.to_site_id.as_str())
            })
            .map(|e| e.edge_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        contexts.insert(
            domain.domain_id.clone(),
            ExternalContext {
                ports,
                halo_edge_ids,
                omitted_contacts,
                omitted_population_expected,
            },
        );
    }
    Ok(contexts)
}
/// Displacements after a ruin (§7.5): one draw per ruined site, towards the
/// nearest site of the same domain, else a port of the origin. Without either
/// there is no move: no site outside the domain can receive households.
fn compile_displacements(
    canon: &GeneralCanon,
    shocks: &[LocalShock],
    contexts: &BTreeMap<String, ExternalContext>,
    baselines: &BTreeMap<String, BTreeMap<i32, u64>>,
    ledger: &mut EventLedger,
) -> Result<Vec<Displacement>, HistoryError> {
    let p = &canon.config.processes;
    let mut displacements = Vec::new();
    for shock in shocks.iter().filter(|s| s.material_damage) {
        let keys = ["region/displacement/v1", shock.effect_id.as_str()];
        if !gate(canon.world_seed, &keys, p.displacement_bps)? {
            continue;
        }
        let origin = &shock.site_id;
        let year = shock.valid.start_year;
        let Some(domain) = canon.domain_of_site(origin) else {
            continue;
        };
        let days = canon.atlas.travel_days_from(origin, year);
        let destination = domain
            .site_ids
            .iter()
            .filter(|s| *s != origin)
            .filter_map(|s| days.get(s).map(|d| (*d, s)))
            .min()
            .map(|(_, s)| s.clone());
        let port = contexts[&domain.domain_id]
            .ports
            .iter()
            .find(|p| &p.entry_site_id == origin && p.valid.contains(year))
            .map(|p| p.port_id.clone());
        let destination_port_id = if destination.is_some() { None } else { port };
        if destination.is_none() && destination_port_id.is_none() {
            continue;
        }
        let mut participants = Vec::new();
        for site in std::iter::once(origin).chain(destination.iter()) {
            participants.push(ObjectRef::Site(
                SemanticAddress::site(&canon.canon_id, site)?.encode(),
            ));
        }
        let event = ledger.add(NewEvent {
            owner: &Owner::site(&canon.canon_id, origin),
            kind: "displacement",
            year,
            end_year: None,
            phase: EventPhase::Residences,
            causes: vec![shock.source_event_id.clone()],
            participants,
            provenance: canon.provenance("region/displacement/v1"),
        })?;
        let population = baseline_at(&baselines[origin], year);
        let moved = population * u64::from(p.displacement_share_bps) / 10_000;
        let tail = event.owned_tail();
        displacements.push(Displacement {
            event_id: event.encode(),
            op_id: format!("op/displacement/{origin}/{}/{}", tail[1], tail[2]),
            origin_site_id: origin.clone(),
            destination_site_id: destination,
            destination_port_id,
            year,
            target_households: (moved / p.expected_household_size).max(1),
        });
    }
    Ok(displacements)
}
/// Separate forecast of §7.4: the marker slope, minus expected extra deaths, plus
/// expected port and displacement flows. It feeds no marker.
fn expected_after_impacts(
    canon: &GeneralCanon,
    baselines: &BTreeMap<String, BTreeMap<i32, u64>>,
    shocks: &[LocalShock],
    displacements: &[Displacement],
    contexts: &BTreeMap<String, ExternalContext>,
) -> BTreeMap<String, BTreeMap<i32, u64>> {
    let size = i128::from(canon.config.processes.expected_household_size);
    let timeline = canon.timeline;
    let ports: Vec<_> = contexts.values().flat_map(|c| &c.ports).collect();
    let mut forecasts = BTreeMap::new();
    for (site, series) in baselines {
        let mut population = i128::from(baseline_at(series, timeline.start_year));
        let mut remainder = 0i128;
        let mut forecast = BTreeMap::new();
        for year in timeline.start_year..timeline.end_year {
            if series.contains_key(&year) {
                forecast.insert(year, population.max(0) as u64);
            }
            let slope =
                i128::from(baseline_at(series, year + 1)) - i128::from(baseline_at(series, year));
            let mut next = population + slope;
            for shock in shocks
                .iter()
                .filter(|s| &s.site_id == site && s.valid.contains(year))
            {
                let bps = shock.effects.additional_mortality_bps.unwrap_or(0);
                next -= population * i128::from(bps) / 10_000;
            }
            let households = population / size;
            for port in ports.iter().filter(|p| &p.entry_site_id == site) {
                if let Some(e) = port.rate_episodes.iter().find(|e| e.valid.contains(year)) {
                    remainder += households
                        * (i128::from(e.incoming_rate_bps) - i128::from(e.outgoing_rate_bps));
                }
            }
            next += remainder.div_euclid(10_000) * size;
            remainder = remainder.rem_euclid(10_000);
            for d in displacements.iter().filter(|d| d.year == year) {
                let moved = i128::from(d.target_households) * size;
                if &d.origin_site_id == site {
                    next -= moved;
                }
                if d.destination_site_id.as_ref() == Some(site) {
                    next += moved;
                }
            }
            population = next.max(0);
        }
        forecasts.insert(site.clone(), forecast);
    }
    forecasts
}

const LEGAL_RULE: &str = "region/legal/v1";
/// One segment of a group's legal history under one jurisdiction.
struct Segment {
    jurisdiction: String,
    status: LegalStatus,
    start: i32,
    end: Option<i32>,
    cause: Option<String>,
    end_cause: Option<String>,
}
/// Dated operations of the legal pass, applied in `(year, order, sequence)` order:
/// control first, then foundations, legal acts and dissolutions.
enum Op {
    Control {
        site_id: String,
        polity_id: String,
        treaty_id: String,
    },
    Found(usize),
    Birth {
        kind: String,
        site_id: String,
        cause_event_id: String,
    },
    Legal {
        polity_id: String,
        kind: &'static str,
        block: i32,
    },
    Dissolve(usize),
}
/// Group slots and legal chains of a region (§23.1, §27.3).
struct RegionRun<'a> {
    canon: &'a GeneralCanon,
    owner: Owner,
    baselines: &'a BTreeMap<String, BTreeMap<i32, u64>>,
    ledger: EventLedger,
    groups: Vec<RegionGroup>,
    slots: BTreeMap<(String, String, i32), u32>,
    segments: BTreeMap<String, Vec<Segment>>,
    ops: BTreeMap<(i32, u8, u32), Op>,
    sequence: u32,
}
impl RegionRun<'_> {
    fn schedule(&mut self, year: i32, order: u8, op: Op) {
        self.sequence += 1;
        self.ops.insert((year, order, self.sequence), op);
    }
    fn adults(&self, site_id: &str, year: i32) -> u64 {
        let share = self
            .canon
            .catalogue
            .engine_defaults
            .expected_adult_share_bps;
        baseline_at(&self.baselines[site_id], year) * share / 10_000
    }
    fn is_available(
        &self,
        archetype: &ArchetypeDefinition,
        site: &SettlementAnchor,
        year: i32,
    ) -> Result<bool, HistoryError> {
        let episode = site.episode_at(year).expect("covering episodes");
        let setting = self.canon.config.setting_profile;
        let overlays = self
            .canon
            .catalogue
            .active_overlays(episode.culture_weights.keys().map(String::as_str), setting)?;
        Ok(archetype.is_available(
            self.adults(&site.site_id, year),
            &site.site_tags,
            setting,
            &overlays,
        ))
    }
    fn alive_count(&self, site_id: &str, archetype: &str, year: i32) -> u32 {
        self.groups
            .iter()
            .filter(|g| {
                g.site_id == site_id
                    && g.entity.archetype_id == archetype
                    && g.entity.existence.contains(year)
            })
            .count() as u32
    }
    fn new_group(
        &mut self,
        archetype: &ArchetypeDefinition,
        site_id: &str,
        origin_block: i32,
        existence: TimeSpan,
    ) -> Result<usize, HistoryError> {
        let key = (site_id.to_owned(), archetype.id.clone(), origin_block);
        let ordinal = self.slots.entry(key).or_insert(0);
        let owner = Owner::site(&self.canon.canon_id, site_id);
        let address = SemanticAddress::group(&owner, &archetype.id, origin_block, *ordinal)?;
        *ordinal += 1;
        let (nature, agency) = if archetype.class == "community" {
            (EntityNature::Community, Agency::None)
        } else {
            (EntityNature::OrganizedGroup, Agency::Delegated)
        };
        self.groups.push(RegionGroup {
            entity: SocialEntity {
                id: address.encode(),
                semantic_address: address,
                kind: archetype.class.clone(),
                archetype_id: archetype.id.clone(),
                nature,
                agency,
                owner_scope: owner.scope().clone(),
                existence,
                culture_refs: principal_culture(&self.canon.atlas, site_id)
                    .into_iter()
                    .collect(),
                institution_ref: None,
                provenance: self.canon.provenance("region/group/v1"),
                name_tree: None,
            },
            site_id: site_id.into(),
        });
        Ok(self.groups.len() - 1)
    }
    /// Own-stock groups (§23.1): the expected stock at bootstrap with uniform ages,
    /// then per block the stock times ten over the lifetime. Each slot is fixed by
    /// its keys, so asking for 1370 never redraws a group born in 1320.
    fn draw_stock_groups(&mut self, region: &str) -> Result<(), HistoryError> {
        let canon = self.canon;
        let seed = canon.world_seed;
        let timeline = canon.timeline;
        let bootstrap = decade_block(timeline.start_year);
        for site in region_sites(canon, region) {
            let id = site.site_id.as_str();
            for archetype in canon.catalogue.archetype_definitions.iter().filter(|a| {
                matches!(a.class.as_str(), "association" | "community")
                    && a.availability.stock_tenths_per_1000_adults > 0
            }) {
                let a = &archetype.availability;
                let lifetime =
                    i64::from(archetype.default_mean_lifetime_years.unwrap_or(30).max(1));
                let max = a.max_per_site.unwrap_or(u32::MAX);
                let stock = |adults: u64| a.stock_tenths_per_1000_adults * adults;
                let life = lifetime.to_string();
                let name = archetype.id.as_str();
                if self.is_available(archetype, site, timeline.start_year)? {
                    let keys = ["region/group/v1", id, name, "bootstrap"];
                    let adults = self.adults(id, timeline.start_year);
                    let count = stochastic_round(seed, &keys, stock(adults), 10_000)?
                        .max(u64::from(a.min_per_site.unwrap_or(0)))
                        .min(u64::from(max));
                    for slot in 0..count {
                        let slot = slot.to_string();
                        let keys = ["region/group/v1", id, name, "age", &slot, &life];
                        let age = uniform_below(seed, &keys, 0, lifetime as u64)? as i32;
                        let start_year = (timeline.start_year - age).max(site.foundation_year);
                        let end_year = (start_year + lifetime as i32).min(timeline.end_year);
                        let existence = TimeSpan {
                            start_year,
                            end_year,
                        };
                        let index = self.new_group(archetype, id, bootstrap, existence)?;
                        self.open_initial(index)?;
                    }
                }
                for block in blocks(timeline) {
                    let at = block.max(timeline.start_year);
                    if !self.is_available(archetype, site, at)? {
                        continue;
                    }
                    let text = block.to_string();
                    let keys = ["region/group/v1", id, name, &text, "flow"];
                    let flow = stock(self.adults(id, at)) * 10;
                    let count = stochastic_round(seed, &keys, flow, 10_000 * lifetime as u64)?;
                    for slot in 0..count {
                        let slot = slot.to_string();
                        let keys = ["region/group/v1", id, name, &text, "date", &slot];
                        let Some(year) = date_in_block(seed, &keys, block, timeline)? else {
                            continue;
                        };
                        if year == timeline.start_year || self.alive_count(id, name, year) >= max {
                            continue;
                        }
                        let existence = TimeSpan {
                            start_year: year,
                            end_year: (year + lifetime as i32).min(timeline.end_year),
                        };
                        let index = self.new_group(archetype, id, block, existence)?;
                        self.schedule(year, 1, Op::Found(index));
                    }
                }
            }
        }
        Ok(())
    }
    fn status_under(
        &self,
        group: &RegionGroup,
        polity: &Polity,
    ) -> Result<LegalStatus, HistoryError> {
        self.canon
            .catalogue
            .initial_status(&group.entity.archetype_id, &polity.history_family_id)
    }
    fn controller(&self, site_id: &str, year: i32) -> Result<&'_ Polity, HistoryError> {
        self.canon
            .controller_at(site_id, year)
            .ok_or_else(|| HistoryError {
                owner: Some(site_id.into()),
                at_year: Some(year),
                ..HistoryError::new(
                    ErrorCode::InvalidLegalStanding,
                    "legal.site_without_controller",
                )
            })
    }
    /// Initial standing at the start of observation, the only one without a cause.
    fn open_initial(&mut self, index: usize) -> Result<(), HistoryError> {
        let start = self.canon.timeline.start_year;
        let group = &self.groups[index];
        let polity = self.controller(&group.site_id, start)?;
        let (id, status) = (group.entity.id.clone(), self.status_under(group, polity)?);
        let jurisdiction = polity.entity.id.clone();
        self.schedule_dissolution(index);
        self.set_status(&id, &jurisdiction, status, start, None);
        Ok(())
    }
    fn schedule_dissolution(&mut self, index: usize) {
        let end = self.groups[index].entity.existence.end_year;
        if end < self.canon.timeline.end_year {
            self.schedule(end, 3, Op::Dissolve(index));
        }
    }
    /// Applies a status. A change under the same jurisdiction chains segments; a
    /// new jurisdiction closes the old chain with the same event as its end cause.
    fn set_status(
        &mut self,
        group_id: &str,
        jurisdiction: &str,
        status: LegalStatus,
        year: i32,
        cause: Option<String>,
    ) {
        let segments = self.segments.entry(group_id.to_owned()).or_default();
        if let Some(last) = segments.last_mut().filter(|s| s.end.is_none()) {
            let same_jurisdiction = last.jurisdiction == jurisdiction;
            if same_jurisdiction && last.status == status {
                return;
            }
            if last.start == year {
                if same_jurisdiction {
                    last.status = status;
                    last.cause = cause.or(last.cause.take());
                    return;
                }
                segments.pop();
            } else {
                last.end = Some(year);
                if !same_jurisdiction {
                    last.end_cause.clone_from(&cause);
                }
            }
        }
        segments.push(Segment {
            jurisdiction: jurisdiction.into(),
            status,
            start: year,
            end: None,
            cause,
            end_cause: None,
        });
    }
    fn close(&mut self, group_id: &str, year: i32, cause: String) {
        let segments = self.segments.entry(group_id.to_owned()).or_default();
        if let Some(last) = segments.last_mut().filter(|s| s.end.is_none()) {
            if last.start == year {
                segments.pop();
                // The previous segment now ends without a successor: it takes the
                // closing cause, unless a change of jurisdiction already gave one.
                if let Some(previous) = segments.last_mut().filter(|s| s.end == Some(year)) {
                    previous.end_cause.get_or_insert(cause);
                }
            } else {
                last.end = Some(year);
                last.end_cause = Some(cause);
            }
        }
    }
    fn is_open_under(&self, group_id: &str, jurisdiction: Option<&str>) -> bool {
        self.segments
            .get(group_id)
            .and_then(|s| s.last())
            .is_some_and(|s| s.end.is_none() && jurisdiction.is_none_or(|j| s.jurisdiction == j))
    }
    fn open_status(&self, group_id: &str) -> Option<LegalStatus> {
        self.segments.get(group_id)?.last().map(|s| s.status)
    }
    /// `foundation` event of a late group; it opens the standing as its cause.
    fn found(&mut self, index: usize, causes: Vec<String>) -> Result<String, HistoryError> {
        let canon = self.canon;
        let group = &self.groups[index];
        let year = group.entity.existence.start_year;
        let site = SemanticAddress::site(&canon.canon_id, &group.site_id)?;
        let id = group.entity.id.clone();
        let event = self.ledger.add(NewEvent {
            owner: &Owner::site(&canon.canon_id, &group.site_id),
            kind: SUBJECT_FOUNDATION_EVENT_KIND,
            year,
            end_year: None,
            phase: EventPhase::InstitutionsAndVacancies,
            causes,
            participants: vec![
                ObjectRef::Entity(id.clone()),
                ObjectRef::Site(site.encode()),
            ],
            provenance: canon.provenance("region/group/v1"),
        })?;
        let group = &self.groups[index];
        let polity = self.controller(&group.site_id, year)?;
        let status = self.status_under(group, polity)?;
        let jurisdiction = polity.entity.id.clone();
        self.schedule_dissolution(index);
        self.set_status(&id, &jurisdiction, status, year, Some(event.encode()));
        Ok(event.encode())
    }
    /// Groups born of an event (§23.1): only `birth_events` create them, with the
    /// same context rules as any archetype.
    fn birth(
        &mut self,
        kind: &str,
        site_id: &str,
        year: i32,
        cause: &str,
    ) -> Result<(), HistoryError> {
        let canon = self.canon;
        let site = canon
            .atlas
            .settlements
            .iter()
            .find(|s| s.site_id == site_id)
            .expect("atlas site");
        for archetype in canon
            .catalogue
            .archetype_definitions
            .iter()
            .filter(|a| a.availability.birth_events.contains(kind))
        {
            let max = archetype.availability.max_per_site.unwrap_or(u32::MAX);
            if !self.is_available(archetype, site, year)?
                || self.alive_count(site_id, &archetype.id, year) >= max
            {
                continue;
            }
            let lifetime = archetype.default_mean_lifetime_years.unwrap_or(30).max(1) as i32;
            let existence = TimeSpan {
                start_year: year,
                end_year: (year + lifetime).min(canon.timeline.end_year),
            };
            let index = self.new_group(archetype, site_id, decade_block(year), existence)?;
            self.found(index, vec![cause.to_owned()])?;
        }
        Ok(())
    }
    /// Re-evaluation at a change of control (§27.6): one `charter` of the new
    /// jurisdiction, caused by the treaty, carries every change on the site.
    fn apply_control(
        &mut self,
        site_id: &str,
        polity_id: &str,
        treaty_id: &str,
        year: i32,
    ) -> Result<(), HistoryError> {
        let canon = self.canon;
        let polity = canon
            .polities
            .iter()
            .find(|p| p.entity.id == polity_id)
            .expect("canon polity");
        let site = SemanticAddress::site(&canon.canon_id, site_id)?;
        let charter = self.ledger.add(NewEvent {
            owner: &self.owner,
            kind: "charter",
            year,
            end_year: None,
            phase: EventPhase::InstitutionsAndVacancies,
            causes: vec![treaty_id.to_owned()],
            participants: vec![
                ObjectRef::Entity(polity_id.to_owned()),
                ObjectRef::Site(site.encode()),
            ],
            provenance: canon.provenance(LEGAL_RULE),
        })?;
        let present: Vec<_> = self
            .groups
            .iter()
            .filter(|g| g.site_id == site_id && self.is_open_under(&g.entity.id, None))
            .map(|g| Ok((g.entity.id.clone(), self.status_under(g, polity)?)))
            .collect::<Result<_, HistoryError>>()?;
        for (id, status) in present {
            self.set_status(&id, polity_id, status, year, Some(charter.encode()));
        }
        Ok(())
    }
    /// Legal act of a jurisdiction. An act finding no target leaves no event:
    /// `edict` proscribes a tolerated archetype, `charter` makes one licit,
    /// `heresy_trial` proscribes a group defined by heresy, `banishment` ends an
    /// open proscribed group.
    fn apply_legal(
        &mut self,
        polity_id: &str,
        kind: &'static str,
        block: i32,
        year: i32,
    ) -> Result<(), HistoryError> {
        let canon = self.canon;
        let alignment = &canon.catalogue.archetype_alignment;
        let candidates: Vec<usize> = (0..self.groups.len())
            .filter(|i| {
                let g = &self.groups[*i].entity;
                let status = self.open_status(&g.id);
                let traits = &alignment[&g.archetype_id];
                g.existence.contains(year)
                    && self.is_open_under(&g.id, Some(polity_id))
                    && match kind {
                        "edict" | "charter" => status == Some(LegalStatus::Tolerated),
                        "heresy_trial" => {
                            traits.defining_tags.contains("heresy") && !traits.jurisdiction_organ
                        }
                        _ => {
                            status == Some(LegalStatus::Proscribed)
                                && traits.default_visibility == Visibility::Open
                                && g.existence.start_year < year
                        }
                    }
            })
            .collect();
        if candidates.is_empty() {
            return Ok(());
        }
        let text = block.to_string();
        let keys = [LEGAL_RULE, polity_id, kind, &text, "target"];
        let pick = uniform_below(canon.world_seed, &keys, 0, candidates.len() as u64)? as usize;
        let chosen = candidates[pick];
        // An edict or a charter rules on the archetype across the jurisdiction.
        let archetype = self.groups[chosen].entity.archetype_id.clone();
        let targets: Vec<usize> = match kind {
            "edict" | "charter" => candidates
                .into_iter()
                .filter(|i| self.groups[*i].entity.archetype_id == archetype)
                .collect(),
            _ => vec![chosen],
        };
        let mut participants = vec![ObjectRef::Entity(polity_id.to_owned())];
        participants.extend(
            targets
                .iter()
                .map(|i| ObjectRef::Entity(self.groups[*i].entity.id.clone())),
        );
        let event = self
            .ledger
            .add(NewEvent {
                owner: &self.owner,
                kind,
                year,
                end_year: None,
                phase: EventPhase::InstitutionsAndVacancies,
                causes: vec![],
                participants,
                provenance: canon.provenance(LEGAL_RULE),
            })?
            .encode();
        for index in targets {
            let id = self.groups[index].entity.id.clone();
            match kind {
                "banishment" => {
                    self.close(&id, year, event.clone());
                    self.groups[index].entity.existence.end_year = year;
                }
                "charter" => {
                    let status = LegalStatus::Licit;
                    self.set_status(&id, polity_id, status, year, Some(event.clone()));
                }
                _ => {
                    let status = LegalStatus::Proscribed;
                    self.set_status(&id, polity_id, status, year, Some(event.clone()));
                }
            }
        }
        if kind == "heresy_trial" {
            let site_id = self.groups[chosen].site_id.clone();
            self.birth(kind, &site_id, year, &event)?;
        }
        Ok(())
    }
    fn dissolve(&mut self, index: usize, year: i32) -> Result<(), HistoryError> {
        let canon = self.canon;
        let group = &self.groups[index];
        let id = group.entity.id.clone();
        if !self.is_open_under(&id, None) {
            return Ok(());
        }
        let site = SemanticAddress::site(&canon.canon_id, &group.site_id)?;
        let event = self.ledger.add(NewEvent {
            owner: &Owner::site(&canon.canon_id, &group.site_id),
            kind: SUBJECT_DISSOLUTION_EVENT_KIND,
            year,
            end_year: None,
            phase: EventPhase::InstitutionsAndVacancies,
            causes: vec![],
            participants: vec![
                ObjectRef::Entity(id.clone()),
                ObjectRef::Site(site.encode()),
            ],
            provenance: canon.provenance("region/group/v1"),
        })?;
        self.close(&id, year, event.encode());
        Ok(())
    }
    fn schedule_world_ops(
        &mut self,
        region: &str,
        displacements: &[Displacement],
    ) -> Result<(), HistoryError> {
        let canon = self.canon;
        let seed = canon.world_seed;
        let timeline = canon.timeline;
        for site in region_sites(canon, region) {
            for episode in canon.site_control[&site.site_id].iter() {
                if let Some(treaty_id) = &episode.cause_event_id {
                    self.schedule(
                        episode.valid.start_year,
                        0,
                        Op::Control {
                            site_id: site.site_id.clone(),
                            polity_id: episode.polity_id.clone(),
                            treaty_id: treaty_id.clone(),
                        },
                    );
                }
            }
        }
        for d in displacements {
            if let Some(site_id) = &d.destination_site_id {
                let op = Op::Birth {
                    kind: "displacement".into(),
                    site_id: site_id.clone(),
                    cause_event_id: d.event_id.clone(),
                };
                self.schedule(d.year, 1, op);
            }
        }
        let inside: BTreeSet<_> = region_sites(canon, region)
            .into_iter()
            .map(|s| s.site_id.as_str())
            .collect();
        for event in canon
            .events
            .iter()
            .filter(|e| e.kind == "diplomatic_contact")
        {
            for participant in &event.participants {
                let address = participant.address()?;
                let site_id = address.fields()[1].as_str();
                if inside.contains(site_id) {
                    let op = Op::Birth {
                        kind: event.kind.clone(),
                        site_id: site_id.to_owned(),
                        cause_event_id: event.id.clone(),
                    };
                    self.schedule(event.year, 1, op);
                }
            }
        }
        for polity in canon.polities.iter().filter(|p| p.region_id == region) {
            for kind in LEGAL_CHANGE_EVENT_KINDS {
                for block in blocks(timeline) {
                    let text = block.to_string();
                    let cell = polity.cell_id.as_str();
                    let keys = |leaf| [LEGAL_RULE, cell, kind, &text, leaf];
                    let bps = canon.config.processes.legal_event_bps;
                    if gate(seed, &keys("gate"), bps)? {
                        if let Some(year) = date_in_block(seed, &keys("date"), block, timeline)? {
                            let polity_id = polity.entity.id.clone();
                            let op = Op::Legal {
                                polity_id,
                                kind,
                                block,
                            };
                            self.schedule(year, 2, op);
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn run_ops(&mut self) -> Result<(), HistoryError> {
        while let Some(((year, _, _), op)) = self.ops.pop_first() {
            match op {
                Op::Control {
                    site_id,
                    polity_id,
                    treaty_id,
                } => self.apply_control(&site_id, &polity_id, &treaty_id, year)?,
                Op::Found(index) => {
                    self.found(index, vec![])?;
                }
                Op::Birth {
                    kind,
                    site_id,
                    cause_event_id,
                } => self.birth(&kind, &site_id, year, &cause_event_id)?,
                Op::Legal {
                    polity_id,
                    kind,
                    block,
                } => self.apply_legal(&polity_id, kind, block, year)?,
                Op::Dissolve(index) => self.dissolve(index, year)?,
            }
        }
        Ok(())
    }
    fn standings(&self) -> Result<Vec<LegalStanding>, HistoryError> {
        let end_year = self.canon.timeline.end_year;
        let mut standings = Vec::new();
        for (group_id, segments) in &self.segments {
            for s in segments {
                let subject = ObjectRef::Entity(group_id.clone());
                let jurisdiction = ObjectRef::Entity(s.jurisdiction.clone());
                let id =
                    SemanticAddress::standing(&self.owner, &subject, &jurisdiction, s.start, 0)?;
                standings.push(LegalStanding {
                    id: id.encode(),
                    subject,
                    jurisdiction,
                    status: s.status,
                    valid: TimeSpan {
                        start_year: s.start,
                        end_year: s.end.unwrap_or(end_year),
                    },
                    cause_event_id: s.cause.clone(),
                    end_cause_event_id: s.end_cause.clone(),
                    provenance: self.canon.provenance(LEGAL_RULE),
                    rule_id: LEGAL_RULE.into(),
                });
            }
        }
        standings.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(standings)
    }
}
/// Rounds `numerator / denominator` to an integer under a fixed key (§23.1).
fn stochastic_round(
    seed: u64,
    parts: &[&str],
    numerator: u64,
    denominator: u64,
) -> Result<u64, HistoryError> {
    let up = uniform_below(seed, parts, 0, denominator)? < numerator % denominator;
    Ok(numerator / denominator + u64::from(up))
}
/// Regional plan, a pure function: calling it for another region first, or twice,
/// changes nothing (§14.1).
pub fn plan_region(canon: &GeneralCanon, region_id: &str) -> Result<RegionPlan, HistoryError> {
    if !canon.baselines.contains_key(region_id) {
        return Err(HistoryError {
            owner: Some(region_id.into()),
            at_year: Some(canon.timeline.start_year),
            ..HistoryError::new(ErrorCode::InvalidAtlas, "region.unknown")
        });
    }
    let baselines = site_baselines(canon, region_id)?;
    let shocks = compile_impacts(canon, region_id)?;
    let contexts = external_contexts(canon, region_id)?;
    let mut ledger = EventLedger::new();
    let displacements = compile_displacements(canon, &shocks, &contexts, &baselines, &mut ledger)?;
    let mut run = RegionRun {
        canon,
        owner: Owner::cell(&canon.canon_id, region_id),
        baselines: &baselines,
        ledger,
        groups: Vec::new(),
        slots: BTreeMap::new(),
        segments: BTreeMap::new(),
        ops: BTreeMap::new(),
        sequence: 0,
    };
    run.draw_stock_groups(region_id)?;
    run.schedule_world_ops(region_id, &displacements)?;
    run.run_ops()?;
    let standings = run.standings()?;
    let mut groups = run.groups;
    groups.sort_by(|a, b| a.entity.id.cmp(&b.entity.id));
    let mut events = run.ledger.events;
    events.sort_by(|a, b| a.id.cmp(&b.id));
    for group in &groups {
        group.entity.validate()?;
    }
    let all_events = [canon.events.as_slice(), events.as_slice()].concat();
    causal_ranks(&all_events)?;
    validate_legal_standings(
        &standings,
        &all_events,
        canon.timeline.start_year,
        canon.timeline.end_year,
    )?;
    let population_expected_after_impacts =
        expected_after_impacts(canon, &baselines, &shocks, &displacements, &contexts);
    Ok(RegionPlan {
        canon_id: canon.canon_id.clone(),
        region_id: region_id.into(),
        site_baselines: baselines,
        population_expected_after_impacts,
        shocks,
        displacements,
        contexts,
        groups,
        events,
        standings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::ids::Owner;
    const PERIOD: TimeSpan = TimeSpan {
        start_year: 1300,
        end_year: 1400,
    };
    fn owner() -> Owner {
        Owner::cell("w", "region")
    }
    // Direct construction, reserved to this module: the budget can be tested without
    // building an atlas of 100,000 sites.
    fn catalogue(n: usize) -> CompleteCandidateCatalogue {
        CompleteCandidateCatalogue {
            owner: owner(),
            target: ImpactTarget::Sites,
            candidates: (0..n)
                .map(|i| (format!("candidate/{i:06}"), PERIOD))
                .collect(),
        }
    }
    fn contract(target: ImpactTarget, mode: ImpactMode) -> ImpactContract {
        let event = SemanticAddress::event(&owner(), "war", 1350, 0).unwrap();
        ImpactContract {
            id: SemanticAddress::impact(&event, "impact-0")
                .unwrap()
                .encode(),
            parent_event_id: event.encode(),
            owner_scope: owner().scope().clone(),
            valid: PERIOD,
            target,
            selector: TargetSelector::All,
            eligibility_time: 1354,
            mode,
            exposure_model: ExposureModel::IndependentTargetV1,
            severity_profile: "war.medium".into(),
            timing_profile: "annual-1".into(),
            channel: "destruction".into(),
            cause: "war".into(),
        }
    }
    #[test]
    fn exact_catalogue_boundary_accepts_100_000_and_refuses_100_001() {
        let c = contract(ImpactTarget::Sites, ImpactMode::ExactCount { count: 3 });
        let accepted = catalogue(MAX_EXACT_CANDIDATES);
        assert_eq!(resolve_impact(42, &c, &accepted).unwrap().targets.len(), 3);
        let refused = catalogue(MAX_EXACT_CANDIDATES + 1);
        assert_eq!(
            resolve_impact(42, &c, &refused).unwrap_err().code,
            ErrorCode::BudgetExceeded
        );
    }
    #[test]
    fn probability_mode_ignores_the_exact_budget() {
        let c = contract(
            ImpactTarget::Sites,
            ImpactMode::Probability { probability_bps: 1 },
        );
        assert!(resolve_impact(42, &c, &catalogue(MAX_EXACT_CANDIDATES + 1)).is_ok());
    }
    #[test]
    fn exact_demographic_impacts_are_rejected_until_s6() {
        for target in [ImpactTarget::People, ImpactTarget::Cohorts] {
            let c = contract(target, ImpactMode::ExactCount { count: 0 });
            assert_eq!(
                resolve_impact(42, &c, &catalogue(0)).unwrap_err().code,
                ErrorCode::UnsupportedCapability
            );
        }
    }
    #[test]
    fn exact_count_is_taken_from_eligible_candidates_only() {
        // Only one of the two candidates exists at the date: K=2 is infeasible and
        // a fraction of one half is computed over a single eligible one.
        let mut full = catalogue(2);
        *full.candidates.values_mut().last().unwrap() = TimeSpan {
            start_year: 1360,
            end_year: 1400,
        };
        let c = contract(ImpactTarget::Sites, ImpactMode::ExactCount { count: 2 });
        assert_eq!(
            resolve_impact(42, &c, &full).unwrap_err().code,
            ErrorCode::ExactImpactInfeasible
        );
        let half = |rounding| ImpactMode::ExactFraction {
            numerator: 1,
            denominator: 2,
            rounding,
        };
        for (rounding, k) in [(RoundingRule::Floor, 0), (RoundingRule::Ceil, 1)] {
            let c = contract(ImpactTarget::Sites, half(rounding));
            assert_eq!(resolve_impact(42, &c, &full).unwrap().targets.len(), k);
        }
    }
    #[test]
    fn invalid_contracts_never_report_an_exact_infeasibility() {
        let full = catalogue(3);
        let bad_probability = contract(
            ImpactTarget::Sites,
            ImpactMode::Probability {
                probability_bps: 10_001,
            },
        );
        let wrong_target = contract(
            ImpactTarget::Assets,
            ImpactMode::Probability { probability_bps: 1 },
        );
        let mut unaddressed_parent = contract(
            ImpactTarget::Sites,
            ImpactMode::Probability { probability_bps: 1 },
        );
        unaddressed_parent.parent_event_id = "event/demo".into();
        let mut foreign_selector = contract(
            ImpactTarget::Sites,
            ImpactMode::Probability { probability_bps: 1 },
        );
        foreign_selector.selector = TargetSelector::Ids(BTreeSet::from(["absent".into()]));
        let zero_fraction = contract(
            ImpactTarget::Sites,
            ImpactMode::ExactFraction {
                numerator: 1,
                denominator: 0,
                rounding: RoundingRule::Floor,
            },
        );
        for c in [
            bad_probability,
            wrong_target,
            unaddressed_parent,
            foreign_selector,
            zero_fraction,
        ] {
            let e = resolve_impact(42, &c, &full).unwrap_err();
            assert_eq!(e.code, ErrorCode::InvalidImpactContract);
            assert_eq!(e.owner.as_deref(), Some("region"));
            assert_eq!(e.at_year, Some(1354));
        }
    }
}
