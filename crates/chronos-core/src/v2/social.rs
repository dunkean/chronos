//! Social packet of a domain (§9.2, §14.1): group slots, memberships, offices, tenures,
//! cohesion and splits, for the whole horizon, from a regional plan and a verified
//! population. Pure: no provider call, no query input, every draw on a semantic key.
use super::atlas::{DemographicDomain, HistoryAtlas, OccupationState, SettlementAnchor};
use super::attributes::{
    capacity_bps, clamp_axes, local_key, person_attributes, three_draws, AttributeContext,
    PersonAttributes, ALIGNMENT_SUBSTREAM,
};
use super::canonical::canonical_value_bytes;
use super::catalog::{ArchetypeDefinition, HistoryCatalogue, SettingProfile, SizeBounds};
use super::display_names::{person_names, PersonName};
use super::general::{
    blocks, date_in_block, fact_provenance, ControlEpisode, EventLedger, GeneralCanon, NewEvent,
    Polity,
};
use super::ids::random_semantics_v2::{domain_seed, uniform_below};
use super::ids::{decade_block, DomainKey, Owner, SemanticAddress};
use super::model::{
    conduct_class, string_enum, Agency, AlignmentAxes, AlignmentEpisode, ConductClass,
    ConductThresholds, CountMeasure, Coverage, EntityNature, EntityState, ErrorCode, EventPhase,
    HistoryError, HistoryEvent, LegalStanding, LegalStatus, Lifecycle, NameTree, ObjectRef,
    ParentRef, PersonRecord, PersonSource, Precision, PrecisionMode, Predicate, Provenance,
    ProvenanceStatus, ResidenceEpisode, SocialAttributeEpisode, SocialEntity, SocialRelation,
    TimeSpan, Visibility,
};
use super::naming::{self, NameContext, Pack, NAMING_RULE};
use super::offices::{
    mayor_office, resolve_office, AppointmentRule, OfficeDefinition, OfficeRequirements,
    OfficeView, SelectionBinding, SelectionProof, Tenure, TenureEnd, TenureMode, COUNCIL_ROLE,
    HEAD_ROLE, MAYOR_ROLE, SINGLE_PERSONAL_CHARGE,
};
use super::population::{
    HouseholdEpisode, PopulationResult, SlotSource, SocialClass, VerifiedPopulation,
};
use super::recruit::{
    alignment_fit, distance_fit, draw_without_replacement, recruitment_weight, Draw, Fits,
};
use super::region::{baseline_at, RegionPlan};
use super::validate::{
    causal_ranks, validate_legal_standings, SUBJECT_DISSOLUTION_EVENT_KIND,
    SUBJECT_FOUNDATION_EVENT_KIND,
};
use super::ALGORITHM_VERSION;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub const SOCIAL_RULE: &str = "chronos-social-packet-1";
pub const SPLIT_RULE_VERSION: &str = "chronos-social-split-1";
const GROUP_KEY: &str = "social/group/v1";
const FRIEND_KEY: &str = "social/friends/v1";
const SPLIT_KEY: &str = "social/split/v1";
const SITE_ANCHOR: &str = "site_occupation_anchor";
const THRESHOLD_ANCHOR: &str = "site_threshold_anchor";
const ACTIVITY_UNIT: &str = "activity_unit";
const FRIEND_CIRCLE: &str = "friend_circle";
const INSTITUTION: &str = "institution";
const ASSOCIATION: &str = "association";
const COMMUNITY: &str = "community";
/// The only archetype whose head passes through kin: the catalogue declares no rule
/// for it, so this is a provisional reading of §10.
const HEREDITARY_ARCHETYPE: &str = "lordship";

/// Frozen canon inputs the social layer reads, carried by the regional plan so that
/// `build_social_packet` keeps its two arguments (§14.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocialInputs {
    pub canon_id: String,
    pub world_seed: u64,
    pub timeline: TimeSpan,
    pub setting: SettingProfile,
    pub provider_algorithm: String,
    pub catalogue: HistoryCatalogue,
    pub atlas: HistoryAtlas,
    pub social_classes: Vec<SocialClass>,
    pub society_ids: BTreeMap<String, String>,
    pub adult_age: i32,
    pub polities: Vec<Polity>,
    pub site_control: BTreeMap<String, Vec<ControlEpisode>>,
    pub domains: Vec<DemographicDomain>,
    pub canon_events: Vec<HistoryEvent>,
    /// Frozen naming input. `None` explicitly selects the declared fallback.
    pub naming_pack: Option<&'static Pack>,
}
impl SocialInputs {
    pub fn from_canon(canon: &GeneralCanon) -> Self {
        Self {
            canon_id: canon.canon_id.clone(),
            world_seed: canon.world_seed,
            timeline: canon.timeline,
            setting: canon.config.setting_profile,
            provider_algorithm: canon.config.provider.algorithm_id.clone(),
            catalogue: canon.catalogue.clone(),
            atlas: canon.atlas.clone(),
            social_classes: canon.config.population.social_classes.clone(),
            society_ids: canon.config.society_ids.clone(),
            adult_age: canon.config.population.kinship.dependent_age as i32,
            polities: canon.polities.clone(),
            site_control: canon.site_control.clone(),
            domains: canon.domains.clone(),
            canon_events: canon.events.clone(),
            naming_pack: Some(naming::frozen_pack()),
        }
    }
}

string_enum!(GroupOrigin {
    RegionPlan,
    SocialPacket
});
string_enum!(MembershipEnd {
    Death,
    Departure,
    Capacity,
    Preempted,
    Dissolution,
    Split,
    Ineligible
});
/// Legacy naming descriptor retained for API compatibility. When `name_tree` is
/// present its provenance identifies Namebench; only `name_fallback` is a fallback.
/// `tone` is the founding class, given once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FallbackName {
    pub kind: String,
    pub tone: ConductClass,
    pub site_id: String,
    pub ordinal: u32,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupRecord {
    pub entity: SocialEntity,
    pub site_id: String,
    pub axes: AlignmentAxes,
    pub conduct_tags: BTreeSet<String>,
    pub visibility: Visibility,
    /// Expected places. Realized members are a distinct measure (§5.2).
    pub target_members: u32,
    pub time_cost_bps: u16,
    pub preemptible: bool,
    pub origin: GroupOrigin,
    pub split_from: Option<String>,
    pub name: FallbackName,
    pub name_fallback: Option<FallbackName>,
}
/// Compact `member_of`, `employed_by` or `affiliated_with` link; `membership_relation`
/// gives the addressed fact. `group` indexes `SocialPacket::groups`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Membership {
    pub group: u32,
    pub person_slot: u32,
    pub predicate: Predicate,
    /// Start of the full fact. `valid` may be clipped by a window; the relation ID
    /// always comes from this year.
    pub opened_year: i32,
    pub valid: TimeSpan,
    pub time_cost_bps: u16,
    pub vacancy_ordinal: u32,
    pub draw_ordinal: u32,
    pub candidates: u32,
    pub closed_by: Option<MembershipEnd>,
    /// Event that opened the link: a split for a transferred member, none for an
    /// ordinary recruitment.
    pub opening_event_id: Option<String>,
    pub closing_event_id: Option<String>,
}
/// Dated affinity inside a circle: a circle never makes everyone friends (§5.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonLink {
    pub from_slot: u32,
    pub to_slot: u32,
    pub predicate: Predicate,
    pub group: u32,
    pub valid: TimeSpan,
}
/// Published change of cohesion at phase 55 (§26.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CohesionRecord {
    pub group: u32,
    pub year: i32,
    pub delta_bps: i32,
    pub cohesion_bps: u16,
    pub years_below_threshold: u32,
    pub source_event_ids: Vec<String>,
    pub vacancy_penalty: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SocialDiagnostic {
    pub code: String,
    pub subject: String,
    pub year: Option<i32>,
}
/// Domain balance at phase 80: affiliations never add people (§18.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SocialYearBalance {
    pub year: i32,
    pub population: u64,
    pub births: u64,
    pub deaths: u64,
    pub incoming: u64,
    pub outgoing: u64,
    pub persons_with_affiliation: u64,
    pub memberships: u64,
    pub vacant_offices: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PersonLife {
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub source: SlotSource,
}
/// View archetypes of a site: households and lineages are read, never drawn (§5.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SiteViews {
    pub household_archetype_id: Option<String>,
    pub lineage_archetype_id: Option<String>,
}
/// A name of an existing provider household or simulated-core lineage. This is
/// naming metadata for a view, not another organization or another population.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonGroupName {
    pub subject_ref: String,
    pub view_kind: String,
    pub archetype_id: String,
    pub site_id: String,
    /// Caller-side family key; the host supplies its surname label to `render`.
    pub surname_ref: String,
    pub name_tree: Option<NameTree>,
    pub name_fallback: Option<FallbackName>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SocialPacket {
    pub canon_id: String,
    pub domain_id: String,
    pub plan_id: String,
    pub plan_fingerprint: String,
    pub result_fingerprint: String,
    pub rule_id: String,
    pub split_rule_version: String,
    pub timeline: TimeSpan,
    /// Exclusive end of what was computed; `timeline.end_year` for a full packet.
    pub horizon_end: i32,
    pub person_thresholds: ConductThresholds,
    pub organization_thresholds: ConductThresholds,
    pub initial_cohesion_bps: u16,
    pub people: Vec<PersonAttributes>,
    pub lives: Vec<PersonLife>,
    pub groups: Vec<GroupRecord>,
    pub site_views: BTreeMap<String, SiteViews>,
    pub person_group_names: Vec<PersonGroupName>,
    /// Display labels by slot, from their own keys: no fact reads them.
    pub person_names: Vec<Option<PersonName>>,
    /// Surname of each family key, for the `surname` refs of name trees.
    pub family_surnames: BTreeMap<String, String>,
    pub memberships: Vec<Membership>,
    pub links: Vec<PersonLink>,
    pub offices: Vec<OfficeDefinition>,
    pub tenures: Vec<Tenure>,
    pub relations: Vec<SocialRelation>,
    pub standings: Vec<LegalStanding>,
    pub events: Vec<HistoryEvent>,
    pub cohesion: Vec<CohesionRecord>,
    pub balance: Vec<SocialYearBalance>,
    pub diagnostics: Vec<SocialDiagnostic>,
}
impl SocialPacket {
    /// Canonical bytes: the builder has already sorted the lists.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, HistoryError> {
        canonical_value_bytes(self)
    }
}

fn social_error(key: &str, owner: &str, year: Option<i32>) -> HistoryError {
    HistoryError {
        owner: Some(owner.into()),
        at_year: year,
        ..HistoryError::new(ErrorCode::InvalidPopulationResult, key)
    }
}
pub(crate) fn founding_group_axes(
    inputs: &SocialInputs,
    group_id: &str,
    archetype: &str,
) -> Result<AlignmentAxes, HistoryError> {
    let a = inputs
        .catalogue
        .archetype_alignment
        .get(archetype)
        .ok_or_else(|| social_error("social.archetype_without_alignment", archetype, None))?;
    let axis = |name, spread: i32| {
        let parts = [ALIGNMENT_SUBSTREAM, "group", name, group_id];
        three_draws(inputs.world_seed, &parts, spread.div_euclid(2))
    };
    clamp_axes(
        a.morality_bps + axis("morality", a.morality_spread_bps)?,
        a.lawfulness_bps + axis("lawfulness", a.lawfulness_spread_bps)?,
    )
}

/// Assign a canonical name once. Person-derived callers supply a stable surname
/// reference, resolved to display text by the host rather than copied into the tree.
pub fn initialize_organization_name(
    inputs: &SocialInputs,
    expander: Option<&naming::Expander<'static>>,
    entity: &mut SocialEntity,
    site: &SettlementAnchor,
    region_id: &str,
    axes: AlignmentAxes,
    surname_ref: Option<&str>,
) -> Result<(), HistoryError> {
    let definition = inputs
        .catalogue
        .archetype_definitions
        .iter()
        .find(|a| a.id == entity.archetype_id)
        .ok_or_else(|| social_error("social.unknown_archetype", &entity.archetype_id, None))?;
    let observed = entity.existence.start_year.max(inputs.timeline.start_year);
    let thresholds = inputs
        .catalogue
        .engine_defaults
        .alignment
        .conduct_class_thresholds
        .organization
        .into();
    let tone = conduct_class(axes, &thresholds);
    if entity.name_tree.is_none() {
        if let Some(pack) = inputs.naming_pack {
            let episode = site.episode_at(observed).expect("covering episodes");
            // Choose the source variant, not the history family. Bootstrap
            // groups use the earliest observed occupation (older data is absent).
            let total: u64 = episode
                .culture_weights
                .values()
                .map(|w| u64::from(*w))
                .sum();
            let mut draw = uniform_below(
                inputs.world_seed,
                &[NAMING_RULE, &entity.id, "culture"],
                0,
                total,
            )?;
            let culture = episode
                .culture_weights
                .iter()
                .find(|(_, w)| {
                    if draw < u64::from(**w) {
                        true
                    } else {
                        draw -= u64::from(**w);
                        false
                    }
                })
                .expect("weighted culture")
                .0;
            let mut refs = BTreeMap::from([
                ("site".into(), site.site_id.clone()),
                ("region".into(), region_id.into()),
            ]);
            if let Some(polity) = inputs
                .site_control
                .get(&site.site_id)
                .and_then(|episodes| episodes.iter().find(|e| e.valid.contains(observed)))
                .map(|e| e.polity_id.as_str())
            {
                refs.insert("polity".into(), polity.into());
            }
            if let Some(surname) = surname_ref {
                refs.insert("surname".into(), surname.into());
            }
            let activities: Vec<_> = episode
                .activity_weights
                .iter()
                .filter(|(id, w)| **w > 0 && definition.activity_ids.contains(*id))
                .collect();
            let total: u64 = activities.iter().map(|(_, w)| u64::from(**w)).sum();
            let activity = if total > 0 {
                let mut draw = uniform_below(
                    inputs.world_seed,
                    &[NAMING_RULE, &entity.id, "activity"],
                    0,
                    total,
                )?;
                activities
                    .into_iter()
                    .find(|(_, w)| {
                        if draw < u64::from(**w) {
                            true
                        } else {
                            draw -= u64::from(**w);
                            false
                        }
                    })
                    .map(|(id, _)| id.clone())
            } else {
                None
            };
            let tone_name = match tone {
                ConductClass::Lawless => "lawless",
                ConductClass::Crooked => "crooked",
                ConductClass::Ordinary => "ordinary",
                ConductClass::Upright => "upright",
                ConductClass::Saintly => "saintly",
            };
            let context = NameContext {
                refs,
                activity,
                tone: Some(tone_name.into()),
            };
            let own;
            let expander = match expander {
                Some(shared) => shared,
                None => {
                    own = naming::Expander::new(pack);
                    &own
                }
            };
            entity.name_tree = match expander.expand_keyed(
                &definition.name_profile,
                culture,
                &context,
                inputs.world_seed,
                &entity.id,
            ) {
                // An unsupported kind is an explicit, synthetic fallback.
                Err(e) if e.constraint == "naming.unknown_kind" => None,
                result => result?,
            };
        }
    }
    Ok(())
}

/// Rational stock rounded under a fixed key (§23.1).
fn round_ratio(seed: u64, parts: &[&str], num: u128, den: u128) -> Result<u64, HistoryError> {
    const SCALE: u128 = 1_000_000;
    let fraction = (num % den) * SCALE / den;
    let up = u128::from(uniform_below(seed, parts, 0, SCALE as u64)?) < fraction;
    Ok((num / den) as u64 + u64::from(up))
}
/// Entry of a `value -> weight` table (`"3": 3000`), under a fixed key.
fn weighted_value(
    seed: u64,
    parts: &[&str],
    table: &BTreeMap<String, u64>,
) -> Result<u32, HistoryError> {
    let mut rows: Vec<(u32, u64)> = table
        .iter()
        .filter_map(|(k, w)| Some((k.parse().ok()?, *w)))
        .collect();
    rows.sort_unstable();
    let total: u64 = rows.iter().map(|(_, w)| w).sum();
    let mut value = uniform_below(seed, parts, 0, total.max(1))?;
    for (key, weight) in rows {
        if value < weight {
            return Ok(key);
        }
        value -= weight;
    }
    Ok(1)
}

struct GroupRuntime {
    archetype: usize,
    site: usize,
    /// Associations and communities recruit within the travel budget; others on site.
    wide: bool,
    open: Vec<u32>,
    /// Member slots of the published state Y−1.
    published: Vec<u32>,
    cohesion: i32,
    below_years: u32,
    next_split_year: i32,
    focus_rank: u32,
    activity_mask: u32,
    kin_bonus: u32,
    required_offices: Vec<usize>,
    vacant_published: bool,
}
struct Selected {
    slot: u32,
    draw: Draw,
    fits: Fits,
    alignment: u32,
    candidates: u32,
    kin: bool,
    founder: bool,
}
struct OpenLink {
    link: usize,
    group: usize,
}
struct Run<'a> {
    region: &'a RegionPlan,
    inputs: &'a SocialInputs,
    result: &'a PopulationResult,
    domain_id: String,
    seed: u64,
    world_seed: u64,
    start: i32,
    end: i32,
    horizon: i32,
    owner_region: Owner,
    sites: Vec<&'a SettlementAnchor>,
    site_index: BTreeMap<&'a str, usize>,
    /// Reachable sites and days, per site, within the recruitment travel budget.
    reach: Vec<Vec<(usize, u64)>>,
    /// Residence per slot as `[first state, end state)` intervals.
    spans: Vec<Vec<(usize, i32, i32)>>,
    /// Present slots per site and year at phase 80, in ID order.
    residents: Vec<Vec<Vec<u32>>>,
    people: Vec<PersonAttributes>,
    activities: Vec<String>,
    /// `skill_initial` of each slot, by index in `activities`.
    skill_initial: Vec<u32>,
    own_activity: Vec<u8>,
    /// Fields of `people` that recruitment reads for every candidate.
    status_ranks: Vec<u32>,
    person_axes: Vec<AlignmentAxes>,
    workforce: Vec<bool>,
    family: Vec<u32>,
    capacity: Vec<u16>,
    max_rank: u32,
    groups: Vec<GroupRecord>,
    rt: Vec<GroupRuntime>,
    fill_order: Vec<usize>,
    ordinals: BTreeMap<(String, String, i32), u32>,
    memberships: Vec<Membership>,
    person_commitments: Vec<Vec<u32>>,
    committed: Vec<u16>,
    preemptible: Vec<u16>,
    links: Vec<PersonLink>,
    open_links: Vec<OpenLink>,
    links_by_person: Vec<Vec<u32>>,
    offices: Vec<OfficeDefinition>,
    office_order: Vec<usize>,
    office_group: Vec<usize>,
    open_tenure: Vec<Option<usize>>,
    tenure_count: Vec<u32>,
    last_holder: Vec<Option<u32>>,
    holds_charge: Vec<bool>,
    tenures: Vec<Tenure>,
    activity_years: Vec<u16>,
    /// Links of each slot with the group being filled, zero outside `count_network`.
    network_links: Vec<u32>,
    network_touched: Vec<u32>,
    ledger: EventLedger,
    standings: Vec<LegalStanding>,
    proscribed: Vec<(usize, String, TimeSpan)>,
    outlaws: Vec<(u32, String, TimeSpan)>,
    charters: BTreeMap<(String, i32), String>,
    ineligible: BTreeMap<String, Rc<BTreeSet<u32>>>,
    cohesion: Vec<CohesionRecord>,
    balance: Vec<SocialYearBalance>,
    relations: Vec<SocialRelation>,
    diagnostics: Vec<SocialDiagnostic>,
    stamp: Vec<u32>,
    stamp_value: u32,
    /// Shared by every name of the packet.
    expander: Option<naming::Expander<'static>>,
}

/// Domain of a verified result, read back from its plan address.
fn domain_of(result: &PopulationResult) -> Result<DomainKey, HistoryError> {
    let address = SemanticAddress::decode(&result.plan_id)?;
    let fields = address.fields();
    let ordinal = fields[3]
        .parse()
        .map_err(|_| social_error("social.plan_id_invalid", &result.plan_id, None))?;
    DomainKey::new(&fields[0], &fields[1], &fields[2], ordinal)
}
/// `[first state, end state)` of a residence: a phase at or before publication counts
/// for its own year, so a same-year visit shows in no state (§3.2).
pub(crate) fn state_span(start: i32, start_phase: u8, end: i32, end_phase: u8) -> (i32, i32) {
    let published = EventPhase::StatePublication as u8;
    (
        start + i32::from(start_phase > published),
        end + i32::from(end_phase > published),
    )
}
impl<'a> Run<'a> {
    fn new(
        region: &'a RegionPlan,
        result: &'a PopulationResult,
        horizon: i32,
    ) -> Result<Self, HistoryError> {
        let inputs = region.social.as_ref();
        let key = domain_of(result)?;
        let domain_id = SemanticAddress::domain(&key)?.encode();
        let domain = inputs
            .domains
            .iter()
            .find(|d| d.domain_id == domain_id)
            .ok_or_else(|| social_error("social.unknown_domain", &domain_id, None))?;
        let seed = domain_seed(inputs.world_seed, &domain_id, ALGORITHM_VERSION);
        let mut sites: Vec<&SettlementAnchor> = domain
            .site_ids
            .iter()
            .filter_map(|id| inputs.atlas.settlements.iter().find(|s| &s.site_id == id))
            .collect();
        sites.sort_by(|a, b| a.site_id.cmp(&b.site_id));
        let site_index: BTreeMap<&str, usize> = sites
            .iter()
            .enumerate()
            .map(|(i, s)| (s.site_id.as_str(), i))
            .collect();
        let (start, end) = (inputs.timeline.start_year, inputs.timeline.end_year);
        let defaults = &inputs.catalogue.engine_defaults;
        let budget = defaults
            .influence_profiles
            .get("recruitment")
            .map_or(0, |p| p.travel_budget_days);
        let reach = sites
            .iter()
            .map(|s| {
                let days = inputs.atlas.travel_days_from(&s.site_id, start);
                sites
                    .iter()
                    .enumerate()
                    .filter_map(|(i, other)| {
                        let d = *days.get(&other.site_id)?;
                        (d <= budget).then_some((i, d))
                    })
                    .collect()
            })
            .collect();
        let table = &result.candidate_table;
        let n = table.len();
        let mut spans = vec![Vec::new(); n];
        let mut first_site: Vec<Option<(&str, i32)>> = vec![None; n];
        let years = (end - start) as usize;
        let mut residents = vec![vec![Vec::new(); years]; sites.len()];
        for residence in &result.residence_index {
            let slot = residence.person_slot as usize;
            let Some(site) = site_index.get(residence.site_id.as_str()).copied() else {
                continue;
            };
            let (from, to) = state_span(
                residence.valid.start_year,
                residence.start_phase,
                residence.valid.end_year,
                residence.end_phase,
            );
            if first_site[slot].is_none() {
                first_site[slot] = Some((sites[site].site_id.as_str(), residence.valid.start_year));
            }
            if from >= to {
                continue;
            }
            spans[slot].push((site, from, to));
            for year in from.max(start)..to.min(end) {
                residents[site][(year - start) as usize].push(residence.person_slot);
            }
        }
        residents
            .iter_mut()
            .flatten()
            .for_each(|list: &mut Vec<u32>| list.sort_unstable());
        let society = &domain.demography_society_id;
        let template = inputs
            .society_ids
            .iter()
            .find(|(_, id)| *id == society)
            .ok_or_else(|| social_error("social.unknown_society", society, None))?
            .0;
        let status_weights = &inputs.catalogue.template(template)?.status_weights;
        let mut diagnostics = Vec::new();
        let people = person_attributes(
            &AttributeContext {
                inputs,
                table,
                domain_id: &domain_id,
                domain: &key,
                seed,
                status_weights,
                first_site: &first_site,
            },
            &mut diagnostics,
        )?;
        let activities: Vec<String> = inputs
            .catalogue
            .required_activity_coverage
            .iter()
            .cloned()
            .collect();
        let skill_initial = people
            .iter()
            .flat_map(|p| {
                let activities = activities.iter();
                activities.map(|a| p.skill_initial.get(a).copied().unwrap_or(0))
            })
            .collect();
        let own_activity = people
            .iter()
            .map(|p| {
                activities
                    .iter()
                    .position(|a| *a == p.activity_id)
                    .map_or(u8::MAX, |i| i as u8)
            })
            .collect();
        let mut family_ids: BTreeMap<&str, u32> = BTreeMap::new();
        let family = table
            .family_key
            .iter()
            .map(|f| {
                let next = family_ids.len() as u32;
                *family_ids.entry(f.as_str()).or_insert(next)
            })
            .collect();
        let capacity = (0..=130)
            .map(|age| capacity_bps(&defaults.work_capacity_bps_by_age, age))
            .collect();
        let mut ledger = EventLedger::new();
        ledger.reserve(&region.events)?;
        let charters = charter_index(region)?;
        let outlaws = person_outlaws(region, &people);
        Ok(Self {
            region,
            inputs,
            result,
            domain_id,
            seed,
            world_seed: inputs.world_seed,
            start,
            end,
            horizon,
            owner_region: Owner::cell(&inputs.canon_id, &region.region_id),
            sites,
            site_index,
            reach,
            spans,
            residents,
            status_ranks: people.iter().map(|p| p.status_rank).collect(),
            person_axes: people.iter().map(|p| p.axes).collect(),
            workforce: people.iter().map(|p| p.workforce).collect(),
            people,
            activities,
            skill_initial,
            own_activity,
            family,
            capacity,
            max_rank: inputs
                .social_classes
                .iter()
                .map(|c| c.provider_rank as u32)
                .max()
                .unwrap_or(0)
                .max(1),
            groups: Vec::new(),
            rt: Vec::new(),
            fill_order: Vec::new(),
            ordinals: BTreeMap::new(),
            memberships: Vec::new(),
            person_commitments: vec![Vec::new(); n],
            committed: vec![0; n],
            preemptible: vec![0; n],
            links: Vec::new(),
            open_links: Vec::new(),
            links_by_person: vec![Vec::new(); n],
            offices: Vec::new(),
            office_order: Vec::new(),
            office_group: Vec::new(),
            open_tenure: Vec::new(),
            tenure_count: Vec::new(),
            last_holder: Vec::new(),
            holds_charge: vec![false; n],
            tenures: Vec::new(),
            activity_years: vec![0; n],
            network_links: vec![0; n],
            network_touched: Vec::new(),
            ledger,
            standings: Vec::new(),
            proscribed: Vec::new(),
            outlaws,
            charters,
            ineligible: BTreeMap::new(),
            cohesion: Vec::new(),
            balance: Vec::new(),
            relations: Vec::new(),
            diagnostics,
            stamp: vec![0; n],
            stamp_value: 0,
            expander: inputs.naming_pack.map(naming::Expander::new),
        })
    }
    fn provenance(&self, rule: &str) -> Provenance {
        fact_provenance(&self.inputs.atlas, rule)
    }
    fn site_at(&self, slot: u32, year: i32) -> Option<usize> {
        self.spans[slot as usize]
            .iter()
            .find(|(_, from, to)| *from <= year && year < *to)
            .map(|(site, _, _)| *site)
    }
    fn age(&self, slot: u32, year: i32) -> i32 {
        year - self.result.candidate_table.birth_year[slot as usize]
    }
    fn capacity_at(&self, slot: u32, year: i32) -> u16 {
        let age = self.age(slot, year).clamp(0, 130) as usize;
        self.capacity[age]
    }
    fn adults(&self, site: usize, year: i32) -> u64 {
        let share = self
            .inputs
            .catalogue
            .engine_defaults
            .expected_adult_share_bps;
        let series = &self.region.site_baselines[&self.sites[site].site_id];
        baseline_at(series, year) * share / 10_000
    }
    fn overlays(&self, site: usize, year: i32) -> Result<BTreeSet<&'a str>, HistoryError> {
        let episode = self.sites[site]
            .episode_at(year)
            .expect("covering episodes");
        self.inputs.catalogue.active_overlays(
            episode.culture_weights.keys().map(String::as_str),
            self.inputs.setting,
        )
    }
    fn family_of_site(
        &self,
        site: usize,
        year: i32,
    ) -> Result<&'a super::catalog::CultureFamily, HistoryError> {
        let episode = self.sites[site]
            .episode_at(year)
            .expect("covering episodes");
        let culture = episode
            .culture_weights
            .iter()
            .max_by_key(|(id, w)| (**w, std::cmp::Reverse(id.as_str())))
            .map(|(id, _)| id.as_str())
            .unwrap_or_default();
        let catalogue = &self.inputs.catalogue;
        catalogue.family(&catalogue.binding(culture)?.history_family_id)
    }
    fn effective_weight(
        &self,
        archetype: &ArchetypeDefinition,
        common: &BTreeSet<String>,
        overlays: &BTreeSet<&str>,
    ) -> u64 {
        let mut weight = u64::from(archetype.availability.weight);
        if common.contains(&archetype.id) {
            weight *= 2;
        }
        for overlay in &self.inputs.catalogue.overlay_definitions {
            if overlays.contains(overlay.id.as_str()) {
                if let Some(modifier) = overlay.archetype_modifiers.get(&archetype.id) {
                    weight *= u64::from(modifier.weight_multiplier);
                }
            }
        }
        weight
    }
    fn effective_size(
        &self,
        archetype: &ArchetypeDefinition,
        overlays: &BTreeSet<&str>,
    ) -> Option<SizeBounds> {
        let overlay = self
            .inputs
            .catalogue
            .overlay_definitions
            .iter()
            .filter(|o| overlays.contains(o.id.as_str()))
            .find_map(|o| o.archetype_modifiers.get(&archetype.id)?.size.clone());
        overlay.or_else(|| archetype.size.clone())
    }
    fn archetype_index(&self, id: &str) -> Option<usize> {
        self.inputs
            .catalogue
            .archetype_definitions
            .iter()
            .position(|a| a.id == id)
    }
    fn archetype(&self, group: usize) -> &'a ArchetypeDefinition {
        &self.inputs.catalogue.archetype_definitions[self.rt[group].archetype]
    }

    // --- Group slots (§23.1) ---

    fn target_size(
        &self,
        archetype: &ArchetypeDefinition,
        site: usize,
        year: i32,
        group_id: &str,
    ) -> Result<u32, HistoryError> {
        let defaults = &self.inputs.catalogue.engine_defaults;
        if archetype.class == FRIEND_CIRCLE {
            let parts = [GROUP_KEY, "size", group_id];
            return weighted_value(
                self.world_seed,
                &parts,
                &defaults.friend_circle_size_weights,
            );
        }
        let overlays = self.overlays(site, year)?;
        let Some(size) = self.effective_size(archetype, &overlays) else {
            return Ok(1);
        };
        let spread = u64::from(size.mean.saturating_sub(size.min)) * 2 + 1;
        let drawn = uniform_below(self.world_seed, &[GROUP_KEY, "size", group_id], 0, spread)?;
        Ok((size.min + drawn as u32).min(size.max))
    }
    fn group_axes(&self, group_id: &str, archetype: &str) -> Result<AlignmentAxes, HistoryError> {
        founding_group_axes(self.inputs, group_id, archetype)
    }
    /// Registers a group of either origin with its runtime state.
    fn adopt(
        &mut self,
        mut entity: SocialEntity,
        site: usize,
        target: Option<u32>,
        origin: GroupOrigin,
        split_from: Option<String>,
    ) -> Result<usize, HistoryError> {
        let inputs = self.inputs;
        let archetype = self
            .archetype_index(&entity.archetype_id)
            .ok_or_else(|| social_error("social.unknown_archetype", &entity.archetype_id, None))?;
        let definition = &inputs.catalogue.archetype_definitions[archetype];
        let traits = &inputs.catalogue.archetype_alignment[&definition.id];
        let axes = self.group_axes(&entity.id, &definition.id)?;
        let observed = entity.existence.start_year.max(self.start);
        let target = match target {
            Some(target) => target,
            None => self.target_size(definition, site, observed, &entity.id)?,
        };
        let family = self.family_of_site(site, observed)?;
        let activity_mask = self
            .activities
            .iter()
            .enumerate()
            .filter(|(_, a)| definition.activity_ids.contains(*a))
            .fold(0, |mask, (i, _)| mask | 1 << i);
        let parts = [GROUP_KEY, "focus", entity.id.as_str()];
        let focus_rank =
            uniform_below(self.world_seed, &parts, 0, u64::from(self.max_rank) + 1)? as u32;
        let ordinal = entity
            .semantic_address
            .owned_tail()
            .last()
            .and_then(|o| o.parse().ok())
            .unwrap_or(0);
        let thresholds = inputs
            .catalogue
            .engine_defaults
            .alignment
            .conduct_class_thresholds
            .organization
            .into();
        let tone = conduct_class(axes, &thresholds);
        initialize_organization_name(
            inputs,
            self.expander.as_ref(),
            &mut entity,
            self.sites[site],
            &self.region.region_id,
            axes,
            None,
        )?;
        let name = FallbackName {
            kind: definition.name_profile.clone(),
            tone,
            site_id: self.sites[site].site_id.clone(),
            ordinal,
            provenance: if entity.name_tree.is_some() {
                Provenance {
                    status: ProvenanceStatus::Derived,
                    source_ref: format!("namebench:{}", naming::generation_fingerprint()),
                    rule_id: NAMING_RULE.into(),
                }
            } else {
                Provenance {
                    status: ProvenanceStatus::Synthetic,
                    source_ref: "name-fallback".into(),
                    rule_id: SOCIAL_RULE.into(),
                }
            },
        };
        let name_fallback = entity.name_tree.is_none().then(|| name.clone());
        self.groups.push(GroupRecord {
            site_id: self.sites[site].site_id.clone(),
            axes,
            conduct_tags: traits.defining_tags.clone(),
            visibility: traits.default_visibility,
            target_members: target,
            time_cost_bps: definition.default_time_cost_bps,
            preemptible: definition.preemptible,
            origin,
            split_from,
            name,
            name_fallback,
            entity,
        });
        self.rt.push(GroupRuntime {
            archetype,
            site,
            wide: matches!(definition.class.as_str(), ASSOCIATION | COMMUNITY),
            open: Vec::new(),
            published: Vec::new(),
            cohesion: inputs.catalogue.engine_defaults.cohesion_mean_bps,
            below_years: 0,
            next_split_year: self.start,
            focus_rank,
            activity_mask,
            kin_bonus: family.parameters.kin_recruitment_bonus,
            required_offices: Vec::new(),
            vacant_published: false,
        });
        Ok(self.groups.len() - 1)
    }
    fn new_group(
        &mut self,
        archetype: &ArchetypeDefinition,
        site: usize,
        block: i32,
        existence: TimeSpan,
        target: Option<u32>,
        split_from: Option<String>,
    ) -> Result<usize, HistoryError> {
        let site_id = self.sites[site].site_id.clone();
        let key = (site_id.clone(), archetype.id.clone(), block);
        let ordinal = self.ordinals.entry(key).or_insert(0);
        let owner = Owner::site(&self.inputs.canon_id, &site_id);
        let address = SemanticAddress::group(&owner, &archetype.id, block, *ordinal)?;
        *ordinal += 1;
        let (nature, agency) = if archetype.class == COMMUNITY || archetype.class == FRIEND_CIRCLE {
            (EntityNature::Community, Agency::None)
        } else if archetype.class == INSTITUTION {
            (EntityNature::OrganizedGroup, Agency::Governed)
        } else {
            (EntityNature::OrganizedGroup, Agency::Delegated)
        };
        let entity = SocialEntity {
            id: address.encode(),
            semantic_address: address,
            kind: archetype.class.clone(),
            archetype_id: archetype.id.clone(),
            nature,
            agency,
            owner_scope: owner.scope().clone(),
            existence,
            culture_refs: self.sites[site]
                .episode_at(existence.start_year.max(self.start))
                .map(|e| e.culture_weights.keys().cloned().collect())
                .unwrap_or_default(),
            institution_ref: None,
            provenance: self.provenance(GROUP_KEY),
            name_tree: None,
        };
        self.adopt(entity, site, target, GroupOrigin::SocialPacket, split_from)
    }
    /// Stock of each drawn archetype at a date, as `(archetype, numerator, denominator)`
    /// (§23.1). Activity units share the expected workers; shared classes share their
    /// stock by weight. Own stocks of associations and communities belong to S2.
    fn stocks(&self, site: usize, year: i32) -> Result<Vec<(usize, u128, u128)>, HistoryError> {
        let anchor = self.sites[site];
        let episode = anchor.episode_at(year).expect("covering episodes");
        if episode.state != OccupationState::Active {
            return Ok(Vec::new());
        }
        let catalogue = &self.inputs.catalogue;
        let defaults = &catalogue.engine_defaults;
        let setting = self.inputs.setting;
        let adults = self.adults(site, year);
        let overlays = self.overlays(site, year)?;
        let family = self.family_of_site(site, year)?;
        let archetypes = &catalogue.archetype_definitions;
        let eligible = |keep: &dyn Fn(&ArchetypeDefinition) -> bool| -> Vec<(usize, u128)> {
            archetypes
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    keep(a) && a.is_available(adults, &anchor.site_tags, setting, &overlays)
                })
                .map(|(i, a)| {
                    let weight = self.effective_weight(a, &family.common_groups, &overlays);
                    (i, u128::from(weight))
                })
                .collect()
        };
        let adults = u128::from(adults);
        let mut stocks = Vec::new();
        let activity_total: u128 = episode
            .activity_weights
            .values()
            .map(|w| u128::from(*w))
            .sum();
        let workers = adults * u128::from(defaults.adult_workforce_participation_bps);
        for (activity, weight) in &episode.activity_weights {
            let types =
                eligible(&|a| a.class == ACTIVITY_UNIT && a.activity_ids.contains(activity));
            let total: u128 = types.iter().map(|(_, w)| w).sum();
            for (i, w) in &types {
                let mean = archetypes[*i].size.as_ref().map_or(1, |s| s.mean.max(1));
                stocks.push((
                    *i,
                    workers * u128::from(*weight) * w,
                    10_000 * activity_total.max(1) * total * u128::from(mean),
                ));
            }
        }
        let formal = u128::from(family.parameters.formal_association_weight);
        let classes = [
            (
                ASSOCIATION,
                u128::from(defaults.formal_associations_per_1000_adults) * formal,
                50,
            ),
            (
                COMMUNITY,
                u128::from(defaults.community_groups_per_1000_adults),
                1,
            ),
            (
                FRIEND_CIRCLE,
                u128::from(defaults.friend_circles_per_1000_adults),
                1,
            ),
        ];
        for (class, rate, divisor) in classes {
            let types = eligible(&|a| {
                a.class == class
                    && a.slot_basis == "expected_active_stock"
                    && a.availability.stock_tenths_per_1000_adults == 0
                    && a.availability.birth_events.is_empty()
            });
            let total: u128 = types.iter().map(|(_, w)| w).sum();
            for (i, w) in &types {
                stocks.push((*i, adults * rate * w, 1_000 * divisor * total));
            }
        }
        for (i, _) in eligible(&|a| {
            a.class == FRIEND_CIRCLE && a.availability.stock_tenths_per_1000_adults > 0
        }) {
            let tenths = archetypes[i].availability.stock_tenths_per_1000_adults;
            stocks.push((i, adults * u128::from(tenths), 10_000));
        }
        Ok(stocks)
    }
    fn lifetime(&self, a: &ArchetypeDefinition, keys: &[&str]) -> Result<i32, HistoryError> {
        let defaults = &self.inputs.catalogue.engine_defaults;
        if a.class == FRIEND_CIRCLE {
            let table = &defaults.friend_circle_lifetime_year_weights;
            return Ok(weighted_value(self.world_seed, keys, table)? as i32);
        }
        Ok(a.default_mean_lifetime_years.unwrap_or(30).max(1) as i32)
    }
    fn alive_on_site(&self, site: usize, archetype: usize, year: i32) -> u32 {
        (0..self.groups.len())
            .filter(|g| {
                self.rt[*g].site == site
                    && self.rt[*g].archetype == archetype
                    && self.groups[*g].entity.existence.contains(year)
            })
            .count() as u32
    }
    /// A community stays present on an active site (§23.1): the heaviest eligible type.
    fn ensure_community(&mut self, site: usize, year: i32, block: i32) -> Result<(), HistoryError> {
        let present = (0..self.groups.len()).any(|g| {
            self.rt[g].site == site
                && self.groups[g].entity.kind == COMMUNITY
                && self.groups[g].entity.existence.contains(year)
        });
        let archetypes = &self.inputs.catalogue.archetype_definitions;
        let best = self
            .stocks(site, year)?
            .into_iter()
            .filter(|(i, _, _)| archetypes[*i].class == COMMUNITY)
            .max_by_key(|(i, num, den)| {
                (
                    num * 1_000_000 / den,
                    std::cmp::Reverse(archetypes[*i].id.as_str()),
                )
            });
        if let (false, Some((i, _, _))) = (present, best) {
            let a = &archetypes[i];
            let existence = TimeSpan {
                start_year: year,
                end_year: (year + self.lifetime(a, &[])?).min(self.end),
            };
            self.new_group(a, site, block, existence, None, None)?;
        }
        Ok(())
    }
    /// Stock with uniform ages at bootstrap, then per block the stock times ten over
    /// the lifetime. A slot is fixed by its keys, like the S2 ones.
    fn draw_stock_slots(&mut self, site: usize) -> Result<(), HistoryError> {
        let inputs = self.inputs;
        let archetypes = &inputs.catalogue.archetype_definitions;
        let seed = self.world_seed;
        let id = self.sites[site].site_id.clone();
        let foundation = self.sites[site].foundation_year;
        let bootstrap = decade_block(self.start);
        for (index, num, den) in self.stocks(site, self.start)? {
            let a = &archetypes[index];
            let name = a.id.as_str();
            let count = round_ratio(seed, &[GROUP_KEY, &id, name, "bootstrap"], num, den)?
                .max(u64::from(a.availability.min_per_site.unwrap_or(0)))
                .min(u64::from(a.availability.max_per_site.unwrap_or(u32::MAX)));
            for slot in 0..count {
                let slot = slot.to_string();
                let life = self.lifetime(a, &[GROUP_KEY, &id, name, "life", &slot])?;
                let keys = [GROUP_KEY, &id, name, "age", &slot];
                let age = uniform_below(seed, &keys, 0, life as u64)? as i32;
                let start_year = (self.start - age).max(foundation);
                let existence = TimeSpan {
                    start_year,
                    end_year: (start_year + life).min(self.end),
                };
                self.new_group(a, site, bootstrap, existence, None, None)?;
            }
        }
        self.ensure_community(site, self.start, bootstrap)?;
        for block in blocks(inputs.timeline) {
            let at = block.max(self.start);
            let text = block.to_string();
            for (index, num, den) in self.stocks(site, at)? {
                let a = &archetypes[index];
                let name = a.id.as_str();
                let mean = u128::from(a.default_mean_lifetime_years.unwrap_or(30).max(1));
                let keys = [GROUP_KEY, &id, name, &text, "flow"];
                let count = round_ratio(seed, &keys, num * 10, den * mean)?;
                for slot in 0..count {
                    let slot = slot.to_string();
                    let keys = [GROUP_KEY, &id, name, &text, "date", &slot];
                    let Some(year) = date_in_block(seed, &keys, block, inputs.timeline)? else {
                        continue;
                    };
                    let max = a.availability.max_per_site.unwrap_or(u32::MAX);
                    if year == self.start
                        || (max < u32::MAX && self.alive_on_site(site, index, year) >= max)
                    {
                        continue;
                    }
                    let life = self.lifetime(a, &[GROUP_KEY, &id, name, &text, "life", &slot])?;
                    let existence = TimeSpan {
                        start_year: year,
                        end_year: (year + life).min(self.end),
                    };
                    self.new_group(a, site, block, existence, None, None)?;
                }
            }
            if at > self.start {
                self.ensure_community(site, at, block)?;
            }
        }
        Ok(())
    }
    fn seats_at(&self, site: usize, year: i32, template: &str) -> Result<u32, HistoryError> {
        let adults = self.adults(site, year);
        let governance = self.inputs.catalogue.governance(template)?;
        Ok(governance
            .collective_seats_thresholds
            .iter()
            .filter(|t| t.min_expected_adults <= adults)
            .map(|t| t.seats)
            .max()
            .unwrap_or(1))
    }
    /// One continuous anchor per site occupation and one secondary anchor per threshold
    /// (§23.1, §25.1). They never follow the stock flow.
    fn plan_anchors(&mut self, site: usize) -> Result<(), HistoryError> {
        let inputs = self.inputs;
        let catalogue = &inputs.catalogue;
        let anchor = self.sites[site];
        let setting = inputs.setting;
        let id = anchor.site_id.as_str();
        let family = self.family_of_site(site, self.start)?;
        let overlays = self.overlays(site, self.start)?;
        let adults = self.adults(site, self.start);
        let eligible: Vec<(&ArchetypeDefinition, u64)> = catalogue
            .archetype_definitions
            .iter()
            .filter(|a| {
                a.slot_basis == SITE_ANCHOR
                    && a.is_available(adults, &anchor.site_tags, setting, &overlays)
            })
            .map(|a| {
                (
                    a,
                    self.effective_weight(a, &family.common_groups, &overlays),
                )
            })
            .collect();
        let total: u64 = eligible.iter().map(|(_, w)| w).sum();
        let closed = anchor
            .occupation_episodes
            .iter()
            .find(|e| e.state == OccupationState::Abandoned)
            .map_or(self.end, |e| e.valid.start_year);
        let mut value =
            uniform_below(self.world_seed, &[GROUP_KEY, id, "anchor"], 0, total.max(1))?;
        let chosen = eligible.iter().find(|(_, w)| {
            let hit = value < *w;
            if !hit {
                value -= *w;
            }
            hit
        });
        let (archetype, _) = chosen.ok_or_else(|| social_error("social.no_anchor", id, None))?;
        let existence = TimeSpan {
            start_year: anchor.foundation_year,
            end_year: closed,
        };
        let seats = self.seats_at(site, self.start, &family.governance_template)?;
        let block = decade_block(anchor.foundation_year);
        self.new_group(archetype, site, block, existence, Some(seats), None)?;
        for a in catalogue
            .archetype_definitions
            .iter()
            .filter(|a| a.slot_basis == THRESHOLD_ANCHOR)
        {
            for year in self.start..closed {
                let overlays = self.overlays(site, year)?;
                let adults = self.adults(site, year);
                if a.is_available(adults, &anchor.site_tags, setting, &overlays) {
                    let existence = TimeSpan {
                        start_year: year,
                        end_year: closed,
                    };
                    self.new_group(a, site, decade_block(year), existence, None, None)?;
                    break;
                }
            }
        }
        Ok(())
    }
    /// Offices of a group (§10, §23.3): a body and a head for a site anchor, one head
    /// otherwise. Other classes hold no charge in this version.
    fn plan_offices(&mut self, group: usize) -> Result<(), HistoryError> {
        let inputs = self.inputs;
        let a = self.archetype(group);
        let site = self.rt[group].site;
        let entity = &self.groups[group].entity;
        let existence = entity.existence;
        let observed = existence.start_year.max(self.start);
        let family = self.family_of_site(site, observed)?;
        let template = inputs.catalogue.governance(&family.governance_template)?;
        let local = &inputs.catalogue.engine_defaults.local_office;
        let is_anchor = a.slot_basis == SITE_ANCHOR;
        let is_threshold = a.slot_basis == THRESHOLD_ANCHOR;
        if !is_anchor && !is_threshold && a.class != ASSOCIATION {
            return Ok(());
        }
        let rule = match template.default_rule.as_str() {
            "seniority" => AppointmentRule::Seniority,
            "appointment" => AppointmentRule::Appointment,
            "collective_body" => AppointmentRule::WeightedElection,
            _ => {
                let error = social_error("social.unknown_rule", &template.id, None);
                return Err(error);
            }
        };
        let (role, rule, term, priority) = if a.id == HEREDITARY_ARCHETYPE {
            (HEAD_ROLE, AppointmentRule::Hereditary, None, 0)
        } else if is_anchor {
            (
                MAYOR_ROLE,
                rule,
                Some(template.default_personal_term_years),
                0,
            )
        } else if is_threshold {
            let term = Some(template.default_personal_term_years);
            (HEAD_ROLE, AppointmentRule::Appointment, term, 1)
        } else {
            (
                HEAD_ROLE,
                rule,
                Some(template.default_personal_term_years),
                2,
            )
        };
        let seats = self.groups[group].target_members;
        let jurisdiction_bound = a.class == INSTITUTION;
        let block = decade_block(existence.start_year);
        let ordinal = self.groups[group].name.ordinal;
        let title_profile = format!("{}/{}", template.id, a.id);
        let add = |run: &mut Self, role: &str, rule, term, collective, cost, required, prio| {
            let site_id = run.sites[site].site_id.clone();
            let owner = Owner::site(&inputs.canon_id, &site_id);
            let slot = format!("{}/{}/{}/{}", a.id, block, ordinal, role);
            let address = SemanticAddress::entity(&owner, "office", block, &slot)?;
            let index = run.offices.len();
            run.offices.push(OfficeDefinition {
                office_id: address.encode(),
                organization_id: run.groups[group].entity.id.clone(),
                site_id: site_id.clone(),
                role_key: role.into(),
                title_profile: title_profile.clone(),
                collective,
                seats: if collective { seats } else { 1 },
                appointment_rule: rule,
                requirements: OfficeRequirements {
                    min_age: template.personal_min_age.max(local.min_age),
                    max_age: template.personal_max_age.min(local.max_age),
                    residence_site_id: site_id,
                    jurisdiction_bound,
                    member_only: a.class == ASSOCIATION,
                },
                term_years: term,
                concurrent_holding_policy: SINGLE_PERSONAL_CHARGE.into(),
                time_cost_bps: cost,
                required,
                priority: prio,
                valid: existence,
                observed_from: observed,
                provenance: run.provenance(SOCIAL_RULE),
            });
            run.office_group.push(group);
            run.open_tenure.push(None);
            run.tenure_count.push(0);
            run.last_holder.push(None);
            if required {
                run.rt[group].required_offices.push(index);
            }
            Ok::<(), HistoryError>(())
        };
        // The head of a time-budgeted association works inside that commitment.
        let head_cost = if a.class == ASSOCIATION && a.default_time_cost_bps > 0 {
            0
        } else {
            local.time_cost_bps
        };
        add(self, role, rule, term, false, head_cost, true, priority)?;
        if is_anchor {
            let body = AppointmentRule::CollectiveBody;
            add(self, COUNCIL_ROLE, body, None, true, 0, false, priority)?;
        }
        Ok(())
    }

    // --- Legal chains (§27.3, §27.4) ---

    fn site_ref(&self, site: usize) -> Result<ObjectRef, HistoryError> {
        let id = &self.sites[site].site_id;
        Ok(ObjectRef::Site(
            SemanticAddress::site(&self.inputs.canon_id, id)?.encode(),
        ))
    }
    fn life_event(
        &mut self,
        group: usize,
        kind: &str,
        year: i32,
        phase: EventPhase,
        causes: Vec<String>,
    ) -> Result<String, HistoryError> {
        let site = self.rt[group].site;
        let owner = Owner::site(&self.inputs.canon_id, &self.sites[site].site_id);
        let participants = vec![
            ObjectRef::Entity(self.groups[group].entity.id.clone()),
            self.site_ref(site)?,
        ];
        let provenance = self.provenance(GROUP_KEY);
        let address = self.ledger.add(NewEvent {
            owner: &owner,
            kind,
            year,
            end_year: None,
            phase,
            causes,
            participants,
            provenance,
        })?;
        Ok(address.encode())
    }
    /// Standing chain of a packet group. Friend circles are informal: they carry none,
    /// and the absence of a standing reads as `licit`.
    fn open_chain(
        &mut self,
        group: usize,
        phase: EventPhase,
        causes: Vec<String>,
    ) -> Result<(), HistoryError> {
        let entity = &self.groups[group].entity;
        if entity.kind == FRIEND_CIRCLE {
            return Ok(());
        }
        let inputs = self.inputs;
        let existence = entity.existence;
        let subject = ObjectRef::Entity(entity.id.clone());
        let archetype = entity.archetype_id.clone();
        let site_id = self.groups[group].site_id.clone();
        let begin = existence.start_year.max(self.start);
        let first_cause = if existence.start_year > self.start {
            let kind = SUBJECT_FOUNDATION_EVENT_KIND;
            Some(self.life_event(group, kind, begin, phase, causes)?)
        } else {
            None
        };
        let closing = if existence.end_year < self.end {
            let kind = SUBJECT_DISSOLUTION_EVENT_KIND;
            let phase = EventPhase::InstitutionsAndVacancies;
            Some(self.life_event(group, kind, existence.end_year, phase, vec![])?)
        } else {
            None
        };
        let episodes: Vec<_> = inputs.site_control[&site_id]
            .iter()
            .filter(|e| e.valid.start_year < existence.end_year && begin < e.valid.end_year)
            .collect();
        let mut merged: Vec<(&str, i32, i32)> = Vec::new();
        for e in episodes {
            let from = e.valid.start_year.max(begin);
            let to = e.valid.end_year.min(existence.end_year);
            match merged.last_mut() {
                Some(last) if last.0 == e.polity_id => last.2 = to,
                _ => merged.push((&e.polity_id, from, to)),
            }
        }
        for (i, (polity_id, from, to)) in merged.iter().enumerate() {
            let charter = |year: i32| -> Result<String, HistoryError> {
                self.charters
                    .get(&(site_id.clone(), year))
                    .cloned()
                    .ok_or_else(|| social_error("social.control_without_charter", &site_id, None))
            };
            let polity = inputs
                .polities
                .iter()
                .find(|p| p.entity.id == *polity_id)
                .ok_or_else(|| social_error("social.unknown_polity", polity_id, None))?;
            let status = inputs
                .catalogue
                .initial_status(&archetype, &polity.history_family_id)?;
            let cause = if i == 0 {
                first_cause.clone()
            } else {
                Some(charter(*from)?)
            };
            let end_cause = if i + 1 < merged.len() {
                Some(charter(*to)?)
            } else {
                closing.clone()
            };
            let jurisdiction = ObjectRef::Entity((*polity_id).to_owned());
            let id =
                SemanticAddress::standing(&self.owner_region, &subject, &jurisdiction, *from, 0)?;
            let valid = TimeSpan {
                start_year: *from,
                end_year: *to,
            };
            if status == LegalStatus::Proscribed {
                self.proscribed
                    .push((group, (*polity_id).to_owned(), valid));
            }
            self.standings.push(LegalStanding {
                id: id.encode(),
                subject: subject.clone(),
                jurisdiction,
                status,
                valid,
                cause_event_id: cause,
                end_cause_event_id: end_cause,
                provenance: fact_provenance(&inputs.atlas, GROUP_KEY),
                rule_id: GROUP_KEY.into(),
            });
        }
        Ok(())
    }
    fn jurisdiction_at(&self, site: usize, year: i32) -> Option<&'a str> {
        self.inputs.site_control[&self.sites[site].site_id]
            .iter()
            .find(|e| e.valid.contains(year))
            .map(|e| e.polity_id.as_str())
    }
    /// Outlaws of a jurisdiction at this date: proscribed persons and known members of
    /// open proscribed groups (§27.5). A covert group's members stay eligible.
    fn ineligible_in(&mut self, jurisdiction: &str, year: i32) -> Rc<BTreeSet<u32>> {
        if let Some(known) = self.ineligible.get(jurisdiction) {
            return known.clone();
        }
        let mut set: BTreeSet<u32> = self
            .outlaws
            .iter()
            .filter(|(_, j, span)| j == jurisdiction && span.contains(year))
            .map(|(slot, _, _)| *slot)
            .collect();
        for (group, j, span) in &self.proscribed {
            if j == jurisdiction
                && span.contains(year)
                && self.groups[*group].visibility == Visibility::Open
            {
                let members = self.rt[*group].open.iter();
                set.extend(members.map(|m| self.memberships[*m as usize].person_slot));
            }
        }
        let set = Rc::new(set);
        self.ineligible.insert(jurisdiction.to_owned(), set.clone());
        set
    }

    // --- Commitments (§9.2, §25.5) ---

    fn end_membership(
        &mut self,
        m: usize,
        year: i32,
        reason: MembershipEnd,
        event: Option<String>,
    ) {
        let record = &mut self.memberships[m];
        record.valid.end_year = year;
        record.closed_by = Some(reason);
        record.closing_event_id = event;
        let (slot, group, cost) = (
            record.person_slot as usize,
            record.group as usize,
            record.time_cost_bps,
        );
        if cost > 0 {
            self.committed[slot] -= cost;
            if self.groups[group].preemptible {
                self.preemptible[slot] -= cost;
            }
            self.person_commitments[slot].retain(|c| *c as usize != m);
        }
        if self.groups[group].entity.kind == FRIEND_CIRCLE {
            let links = &mut self.links;
            self.open_links.retain(|open| {
                let link = &mut links[open.link];
                let touches = open.group == group
                    && (link.from_slot as usize == slot || link.to_slot as usize == slot);
                if touches {
                    link.valid.end_year = year;
                }
                !touches
            });
        }
    }
    /// Closes expired commitments before anything else (§9.2): deaths, departures,
    /// dissolutions and capacity lost with age.
    fn close_commitments(&mut self, year: i32) {
        for group in 0..self.groups.len() {
            if self.rt[group].open.is_empty() {
                continue;
            }
            let ended = !self.groups[group].entity.existence.contains(year);
            let open = std::mem::take(&mut self.rt[group].open);
            let mut kept = Vec::with_capacity(open.len());
            for m in open {
                let slot = self.memberships[m as usize].person_slot;
                let cost = self.memberships[m as usize].time_cost_bps;
                let here = self.site_at(slot, year);
                let reason = if ended {
                    Some(MembershipEnd::Dissolution)
                } else if here.is_none_or(|s| !self.in_reach(group, s)) {
                    let table = &self.result.candidate_table;
                    let dead = table.death_year[slot as usize].is_some_and(|d| d <= year);
                    Some(if dead {
                        MembershipEnd::Death
                    } else {
                        MembershipEnd::Departure
                    })
                } else if cost > 0 && self.committed[slot as usize] > self.capacity_at(slot, year) {
                    Some(MembershipEnd::Capacity)
                } else {
                    None
                };
                match reason {
                    Some(reason) => self.end_membership(m as usize, year, reason, None),
                    None => kept.push(m),
                }
            }
            self.rt[group].open = kept;
        }
    }
    /// A body of the jurisdiction refuses its outlaws like its offices do (§27.5): a
    /// seat closes at the boundary where its holder becomes one.
    fn close_barred_seats(&mut self, year: i32) {
        for group in 0..self.groups.len() {
            if self.groups[group].entity.kind != INSTITUTION || self.rt[group].open.is_empty() {
                continue;
            }
            let Some(jurisdiction) = self.jurisdiction_at(self.rt[group].site, year) else {
                continue;
            };
            let barred = self.ineligible_in(jurisdiction, year);
            let open = std::mem::take(&mut self.rt[group].open);
            let mut kept = Vec::with_capacity(open.len());
            for m in open {
                if barred.contains(&self.memberships[m as usize].person_slot) {
                    self.end_membership(m as usize, year, MembershipEnd::Ineligible, None);
                } else {
                    kept.push(m);
                }
            }
            self.rt[group].open = kept;
        }
    }
    /// Who cannot enter this group without losing a charge: holders of offices and
    /// seats of a jurisdiction that proscribes it. A covert group stays open to them.
    fn charge_conflicts(&self, group: usize, year: i32) -> BTreeSet<u32> {
        let mut holders = BTreeSet::new();
        if self.groups[group].visibility != Visibility::Open {
            return holders;
        }
        for (_, jurisdiction, _) in self
            .proscribed
            .iter()
            .filter(|(g, _, span)| *g == group && span.contains(year))
        {
            let rules = |site: usize| self.jurisdiction_at(site, year) == Some(jurisdiction);
            for (office, def) in self.offices.iter().enumerate() {
                let site = self.site_index[def.requirements.residence_site_id.as_str()];
                if def.requirements.jurisdiction_bound && rules(site) {
                    let tenure = self.open_tenure[office].map(|t| &self.tenures[t]);
                    holders.extend(tenure.and_then(|t| t.person_slot));
                }
            }
            for (body, record) in self.groups.iter().enumerate() {
                if record.entity.kind == INSTITUTION && rules(self.rt[body].site) {
                    let seats = self.rt[body].open.iter();
                    holders.extend(seats.map(|m| self.memberships[*m as usize].person_slot));
                }
            }
        }
        holders
    }
    fn in_reach(&self, group: usize, site: usize) -> bool {
        let rt = &self.rt[group];
        if rt.wide {
            self.reach[rt.site].iter().any(|(s, _)| *s == site)
        } else {
            rt.site == site
        }
    }
    /// Links and affiliations of each person with a group in the published state Y−1;
    /// zero for a foundation (§25.5).
    fn count_network(&mut self, group: usize, year: i32) {
        let previous = year - 1;
        let counts = &mut self.network_links;
        let touched = &mut self.network_touched;
        let mut add = |slot: u32| {
            if counts[slot as usize] == 0 {
                touched.push(slot);
            }
            counts[slot as usize] += 1;
        };
        for slot in &self.rt[group].published {
            add(*slot);
            for link in &self.links_by_person[*slot as usize] {
                let link = &self.links[*link as usize];
                if link.valid.contains(previous) {
                    add(if link.from_slot == *slot {
                        link.to_slot
                    } else {
                        link.from_slot
                    });
                }
            }
        }
    }
    fn clear_network(&mut self) {
        for slot in self.network_touched.drain(..) {
            self.network_links[slot as usize] = 0;
        }
    }
    fn skill_of(&self, slot: u32, activity: usize) -> u32 {
        let defaults = &self.inputs.catalogue.engine_defaults;
        let initial = self.skill_initial[slot as usize * self.activities.len() + activity];
        let years = if usize::from(self.own_activity[slot as usize]) == activity {
            u32::from(self.activity_years[slot as usize])
        } else {
            0
        };
        (initial + defaults.skill_gain_per_activity_year * years).min(1_000)
    }
    fn best_skill(&self, slot: u32, mask: u32) -> u32 {
        let own = usize::from(self.own_activity[slot as usize]);
        if mask == 0 {
            return if own < self.activities.len() {
                self.skill_of(slot, own)
            } else {
                0
            };
        }
        (0..self.activities.len())
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| self.skill_of(slot, i))
            .max()
            .unwrap_or(0)
    }
    fn fits(&self, group: usize, slot: u32, days: u64, kin: &[u32]) -> Fits {
        let defaults = &self.inputs.catalogue.engine_defaults;
        let rt = &self.rt[group];
        let own = u32::from(self.own_activity[slot as usize]);
        let gap = self.status_ranks[slot as usize].abs_diff(rt.focus_rank);
        let links = self.network_links[slot as usize];
        Fits {
            skill: self.best_skill(slot, rt.activity_mask),
            activity: if own < 32 && rt.activity_mask & (1 << own) != 0 {
                1_000
            } else {
                0
            },
            status: 1_000 - 1_000 * gap.min(self.max_rank) / self.max_rank,
            distance: distance_fit(defaults.travel_scale_days, days),
            kin: if kin.binary_search(&self.family[slot as usize]).is_ok() {
                1_000
            } else {
                0
            },
            network: (defaults.network_bonus_per_prior_link * links)
                .min(defaults.network_bonus_max),
        }
    }
    fn member_families(&self, group: usize) -> Vec<u32> {
        let mut families: Vec<u32> = self.rt[group]
            .open
            .iter()
            .map(|m| self.family[self.memberships[*m as usize].person_slot as usize])
            .collect();
        families.sort_unstable();
        families.dedup();
        families
    }

    // --- Offices (§10, §25.5) ---

    fn holder_problem(&mut self, office: usize, slot: u32, year: i32) -> Option<TenureEnd> {
        let def = &self.offices[office];
        let site = self.site_index[def.requirements.residence_site_id.as_str()];
        let (max_age, bound) = (
            def.requirements.max_age,
            def.requirements.jurisdiction_bound,
        );
        let member_only = def.requirements.member_only;
        let group = self.office_group[office];
        let here = self.site_at(slot, year);
        let present = if member_only {
            self.is_member(group, slot)
        } else {
            here == Some(site)
        };
        if !present {
            let table = &self.result.candidate_table;
            let dead = table.death_year[slot as usize].is_some_and(|d| d <= year);
            return Some(if dead {
                TenureEnd::Death
            } else if member_only && here.is_some_and(|s| self.in_reach(group, s)) {
                TenureEnd::Ineligible
            } else {
                TenureEnd::Departure
            });
        }
        if self.age(slot, year) > max_age as i32 {
            return Some(TenureEnd::Ineligible);
        }
        if bound {
            if let Some(jurisdiction) = self.jurisdiction_at(site, year) {
                if self.ineligible_in(jurisdiction, year).contains(&slot) {
                    return Some(TenureEnd::Ineligible);
                }
            }
        }
        None
    }
    fn is_member(&self, group: usize, slot: u32) -> bool {
        self.rt[group]
            .open
            .iter()
            .any(|m| self.memberships[*m as usize].person_slot == slot)
    }
    fn select_holder(
        &mut self,
        office: usize,
        year: i32,
    ) -> Result<Option<Selected>, HistoryError> {
        let def = self.offices[office].clone();
        let site = self.site_index[def.requirements.residence_site_id.as_str()];
        let barred = match self
            .jurisdiction_at(site, year)
            .filter(|_| def.requirements.jurisdiction_bound)
        {
            Some(j) => self.ineligible_in(j, year),
            None => Rc::default(),
        };
        let group = self.office_group[office];
        let member_only = def.requirements.member_only;
        // A founder comes from the residents; afterwards only members qualify, and an
        // association without an eligible member stays vacant.
        let founder =
            member_only && self.tenure_count[office] == 0 && self.rt[group].open.is_empty();
        let pool: Vec<u32> = if member_only && !founder {
            let mut members: Vec<u32> = self.rt[group]
                .open
                .iter()
                .map(|m| self.memberships[*m as usize].person_slot)
                .collect();
            members.sort_unstable();
            members
        } else {
            self.residents[site][(year - self.start) as usize].clone()
        };
        // Time the group itself takes and a preemption must not release.
        let own_cost = if founder || (member_only && self.groups[group].preemptible) {
            self.groups[group].time_cost_bps
        } else {
            0
        };
        let candidates: Vec<u32> = pool
            .into_iter()
            .filter(|slot| {
                let age = self.age(*slot, year);
                let index = *slot as usize;
                let protected = self.committed[index] - self.preemptible[index];
                age >= def.requirements.min_age as i32
                    && age <= def.requirements.max_age as i32
                    && !self.holds_charge[index]
                    && self.capacity_at(*slot, year) >= protected + def.time_cost_bps + own_cost
                    && !barred.contains(slot)
            })
            .collect();
        if candidates.is_empty() {
            return Ok(None);
        }
        let founding = self.groups[group].entity.existence.start_year == year;
        if !founding {
            self.count_network(group, year);
        }
        let kin = self.member_families(group);
        let axes = self.groups[group].axes;
        let scored: Vec<(Fits, u32, u64)> = candidates
            .iter()
            .map(|slot| {
                let mut fits = self.fits(group, *slot, 0, &kin);
                let rank = self.status_ranks[*slot as usize];
                fits.status = 1_000 * rank.min(self.max_rank) / self.max_rank;
                let alignment = alignment_fit(self.person_axes[*slot as usize], axes);
                let bonus = self.rt[group].kin_bonus;
                (fits, alignment, recruitment_weight(&fits, bonus, alignment))
            })
            .collect();
        self.clear_network();
        let mut weights: Vec<u64> = scored.iter().map(|(_, _, w)| *w).collect();
        let table = &self.result.candidate_table;
        let mut kin_rule = false;
        match def.appointment_rule {
            AppointmentRule::Seniority => {
                let oldest = candidates
                    .iter()
                    .map(|s| table.birth_year[*s as usize])
                    .min()
                    .expect("not empty");
                for (i, slot) in candidates.iter().enumerate() {
                    if table.birth_year[*slot as usize] != oldest {
                        weights[i] = 0;
                    }
                }
            }
            AppointmentRule::Hereditary => {
                // Kin of the last holder as the table exposes them; a child is no
                // automatic heir, and a projected family stops at its close links.
                if let Some(holder) = self.last_holder[office] {
                    let children = &self.result.children_index[holder as usize];
                    let family = self.family[holder as usize];
                    let is_kin = |slot: u32| {
                        children.contains(&slot) || self.family[slot as usize] == family
                    };
                    if candidates.iter().any(|s| is_kin(*s)) {
                        kin_rule = true;
                        for (i, slot) in candidates.iter().enumerate() {
                            if !is_kin(*slot) {
                                weights[i] = 0;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        let ordinal = self.tenure_count[office];
        let draws =
            draw_without_replacement(self.seed, &def.office_id, year, ordinal, &mut weights, 1)?;
        Ok(draws.first().map(|draw| {
            let (fits, alignment, _) = scored[draw.index];
            Selected {
                slot: candidates[draw.index],
                draw: *draw,
                fits,
                alignment,
                candidates: candidates.len() as u32,
                kin: kin_rule,
                founder,
            }
        }))
    }
    fn end_tenure(&mut self, office: usize, year: i32, reason: TenureEnd) {
        if let Some(tenure) = self.open_tenure[office].take() {
            let record = &mut self.tenures[tenure];
            record.valid.end_year = year;
            record.closed_by = Some(reason);
            if let Some(slot) = record.person_slot {
                let cost = self.offices[office].time_cost_bps;
                self.committed[slot as usize] -= cost;
                self.holds_charge[slot as usize] = false;
                self.last_holder[office] = Some(slot);
            }
        }
    }
    /// Closes the jobs a charge displaces, traced before the tenure opens (§25.5).
    fn preempt_jobs(
        &mut self,
        office: usize,
        slot: u32,
        year: i32,
        founder: bool,
    ) -> Result<(), HistoryError> {
        let kept = self.office_group[office];
        let joining = if founder {
            self.groups[kept].time_cost_bps
        } else {
            0
        };
        let cost = self.offices[office].time_cost_bps + joining;
        let capacity = self.capacity_at(slot, year);
        while capacity < self.committed[slot as usize] + cost {
            let index = slot as usize;
            let Some(m) = self.person_commitments[index].iter().copied().find(|m| {
                let group = self.memberships[*m as usize].group as usize;
                self.groups[group].preemptible && group != kept
            }) else {
                return Err(social_error(
                    "social.charge_over_capacity",
                    &self.domain_id,
                    Some(year),
                ));
            };
            let group = self.memberships[m as usize].group as usize;
            let site = self.rt[group].site;
            let owner = Owner::site(&self.inputs.canon_id, &self.sites[site].site_id);
            let provenance = self.provenance(SOCIAL_RULE);
            let event = self.ledger.add(NewEvent {
                owner: &owner,
                kind: "employment_preemption",
                year,
                end_year: None,
                phase: EventPhase::Assignments,
                causes: vec![],
                participants: vec![
                    ObjectRef::PersonSlot(self.people[index].person_id.clone()),
                    ObjectRef::Entity(self.groups[group].entity.id.clone()),
                    ObjectRef::Office(self.offices[office].office_id.clone()),
                ],
                provenance,
            })?;
            self.rt[group].open.retain(|o| *o != m);
            self.end_membership(
                m as usize,
                year,
                MembershipEnd::Preempted,
                Some(event.encode()),
            );
        }
        Ok(())
    }
    fn open_tenure_record(
        &mut self,
        office: usize,
        year: i32,
        selected: Option<Selected>,
    ) -> Result<(), HistoryError> {
        let def = &self.offices[office];
        let ordinal = self.tenure_count[office];
        let office_id = def.office_id.clone();
        let site_id = def.site_id.clone();
        let rule = def.appointment_rule;
        let founded_now = def.valid.start_year == year;
        let first = ordinal == 0;
        let address = SemanticAddress::decode(&office_id)?;
        let tenure_id = SemanticAddress::tenure(&address, ordinal)?.encode();
        let binding = match &selected {
            None => SelectionBinding::NoCandidate,
            Some(_) if first && founded_now => SelectionBinding::FoundingSelection,
            Some(_) if first => SelectionBinding::InitialObservedBinding,
            Some(s) if s.kin => SelectionBinding::KinSuccession,
            Some(_) => SelectionBinding::Succession,
        };
        // The first observed holder cites no election: nothing is known before (§26.4).
        let cause = if binding == SelectionBinding::InitialObservedBinding {
            None
        } else {
            let owner = Owner::site(&self.inputs.canon_id, &site_id);
            let mut participants = vec![ObjectRef::Office(office_id.clone())];
            if let Some(s) = &selected {
                let person = self.people[s.slot as usize].person_id.clone();
                participants.push(ObjectRef::PersonSlot(person));
            }
            let provenance = self.provenance(SOCIAL_RULE);
            let event = self.ledger.add(NewEvent {
                owner: &owner,
                kind: if selected.is_some() {
                    "office_selection"
                } else {
                    "office_vacancy"
                },
                year,
                end_year: None,
                phase: EventPhase::Assignments,
                causes: vec![],
                participants,
                provenance,
            })?;
            Some(event.encode())
        };
        let (draw, fits, alignment, candidates) = selected
            .as_ref()
            .map_or((None, Fits::default(), 0, 0), |s| {
                (Some(s.draw), s.fits, s.alignment, s.candidates)
            });
        let slot = selected.as_ref().map(|s| s.slot);
        let proof = SelectionProof {
            rule,
            binding,
            year,
            vacancy_ordinal: ordinal,
            draw_ordinal: draw.map_or(0, |d| d.draw_ordinal),
            candidates,
            total_weight: draw.map_or(0, |d| d.total_weight),
            selected_weight: draw.map_or(0, |d| d.weight),
            draw: draw.map_or(0, |d| d.value),
            fits,
            alignment_fit: alignment,
        };
        if let Some(slot) = slot {
            self.committed[slot as usize] += self.offices[office].time_cost_bps;
            self.holds_charge[slot as usize] = true;
        }
        self.tenures.push(Tenure {
            tenure_id,
            office_id,
            tenure_ordinal: ordinal,
            person_id: slot.map(|s| self.people[s as usize].person_id.clone()),
            person_slot: slot,
            valid: TimeSpan {
                start_year: year,
                end_year: self.horizon,
            },
            mode: if slot.is_some() {
                TenureMode::Effective
            } else {
                TenureMode::Vacant
            },
            selection_proof: proof,
            cause_event_id: cause,
            closed_by: None,
        });
        self.open_tenure[office] = Some(self.tenures.len() - 1);
        self.tenure_count[office] += 1;
        Ok(())
    }
    /// Phase 50 for one office: what no longer holds closes at this boundary, so that
    /// no selection of the year sees a charge that is about to end.
    fn close_office(&mut self, office: usize, year: i32) {
        let def = &self.offices[office];
        if def.collective || year < def.observed_from {
            return;
        }
        let term = def.term_years;
        if !def.valid.contains(year) {
            self.end_tenure(office, year, TenureEnd::Dissolution);
            return;
        }
        if let Some(index) = self.open_tenure[office] {
            let tenure = &self.tenures[index];
            let started = tenure.valid.start_year;
            if let Some(slot) = tenure.person_slot {
                let problem = self.holder_problem(office, slot, year);
                let expired = term.is_some_and(|t| year >= started + t as i32);
                if let Some(reason) = problem.or(expired.then_some(TenureEnd::TermEnd)) {
                    self.end_tenure(office, year, reason);
                }
            }
        }
    }
    /// Phase 60 for one office, before any other recruitment: one look for a holder. A
    /// vacancy opens at the boundary and stays open until a candidate exists (§9.2).
    fn assign_office(&mut self, office: usize, year: i32) -> Result<(), HistoryError> {
        let def = &self.offices[office];
        if def.collective || year < def.observed_from || !def.valid.contains(year) {
            return Ok(());
        }
        if let Some(index) = self.open_tenure[office] {
            if self.tenures[index].person_slot.is_some() {
                return Ok(());
            }
        }
        let selected = self.select_holder(office, year)?;
        match (selected, self.open_tenure[office]) {
            (None, Some(_)) => {}
            (None, None) => self.open_tenure_record(office, year, None)?,
            (Some(s), _) => {
                self.end_tenure(office, year, TenureEnd::Filled);
                let (slot, founder, pool) = (s.slot, s.founder, s.candidates);
                self.preempt_jobs(office, slot, year, founder)?;
                self.open_tenure_record(office, year, Some(s))?;
                if founder {
                    let group = self.office_group[office];
                    let cost = self.groups[group].time_cost_bps;
                    self.join(
                        group,
                        slot,
                        year,
                        Predicate::MemberOf,
                        cost,
                        Ok((0, 0, pool)),
                    );
                }
            }
        }
        Ok(())
    }

    // --- Cohesion and split (§26.5, §26.6) ---

    fn run_cohesion(&mut self, year: i32) -> Result<(), HistoryError> {
        if year == self.start {
            return Ok(());
        }
        let inputs = self.inputs;
        let effects = inputs.catalogue.engine_defaults.cohesion_effects;
        let trigger = &inputs.catalogue.engine_defaults.split_trigger;
        let mut crises: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
        for shock in self.region.shocks.iter().filter(|s| s.valid.contains(year)) {
            if let Some(site) = self.site_index.get(shock.site_id.as_str()) {
                crises
                    .entry(shock.source_event_id.as_str())
                    .or_default()
                    .insert(*site);
            }
        }
        for group in 0..self.groups.len() {
            let existence = self.groups[group].entity.existence;
            if !existence.contains(year) || !existence.contains(year - 1) {
                continue;
            }
            let members = &self.rt[group].published;
            let mut applied = Vec::new();
            if !members.is_empty() {
                for (event, sites) in &crises {
                    let exposed = members
                        .iter()
                        .filter(|slot| {
                            self.site_at(**slot, year - 1)
                                .is_some_and(|s| sites.contains(&s))
                        })
                        .count() as u32;
                    if exposed * 10_000 / members.len() as u32
                        >= effects.exposure_member_share_min_bps
                        && applied.len() < effects.max_crises_per_year
                    {
                        applied.push((*event).to_owned());
                    }
                }
            }
            let vacancy = self.rt[group].vacant_published;
            let delta = if applied.is_empty() && !vacancy {
                effects.recovery_delta_bps_per_unpenalized_year
            } else {
                applied.len() as i32 * effects.crisis_delta_bps_per_event_year
                    + if vacancy {
                        effects.required_office_vacancy_delta_bps_per_year
                    } else {
                        0
                    }
            };
            let rt = &mut self.rt[group];
            let before = rt.cohesion;
            rt.cohesion = (before + delta).clamp(0, 10_000);
            rt.below_years = if rt.cohesion < trigger.cohesion_below_bps {
                rt.below_years + 1
            } else {
                0
            };
            if rt.cohesion != before {
                self.cohesion.push(CohesionRecord {
                    group: group as u32,
                    year,
                    delta_bps: rt.cohesion - before,
                    cohesion_bps: rt.cohesion as u16,
                    years_below_threshold: rt.below_years,
                    source_event_ids: applied,
                    vacancy_penalty: vacancy,
                });
            }
            self.consider_split(group, year)?;
        }
        Ok(())
    }
    fn consider_split(&mut self, group: usize, year: i32) -> Result<(), HistoryError> {
        let inputs = self.inputs;
        let trigger = &inputs.catalogue.engine_defaults.split_trigger;
        let rt = &self.rt[group];
        let archetype = self.archetype(group);
        if !trigger.eligible_classes.contains(&archetype.class)
            || archetype.slot_basis == SITE_ANCHOR
            || archetype.slot_basis == THRESHOLD_ANCHOR
            || rt.below_years < trigger.consecutive_years
            || year < rt.next_split_year
            || rt.open.len() < trigger.min_realized_members
        {
            return Ok(());
        }
        let id = self.groups[group].entity.id.clone();
        let text = year.to_string();
        let parts = [SPLIT_KEY, id.as_str(), text.as_str(), SPLIT_RULE_VERSION];
        if uniform_below(self.world_seed, &parts, 0, 10_000)?
            >= u64::from(trigger.annual_probability_bps)
        {
            return Ok(());
        }
        let site = rt.site;
        let life = archetype.default_mean_lifetime_years.unwrap_or(30).max(1) as i32;
        let existence = TimeSpan {
            start_year: year,
            end_year: (year + life).min(self.end),
        };
        let open = self.rt[group].open.clone();
        let moved_count = open.len() * trigger.new_group_member_share_bps as usize / 10_000;
        let mut order = Vec::with_capacity(open.len());
        for m in &open {
            let slot = self.memberships[*m as usize].person_slot;
            let rank = uniform_below(self.world_seed, &parts, u64::from(slot) + 1, u64::MAX)?;
            order.push((rank, slot, *m));
        }
        order.sort_unstable();
        let new_group = self.new_group(
            archetype,
            site,
            decade_block(year),
            existence,
            Some(moved_count as u32),
            Some(id.clone()),
        )?;
        let owner = Owner::site(&inputs.canon_id, &self.sites[site].site_id);
        let provenance = self.provenance(SPLIT_KEY);
        let split = self
            .ledger
            .add(NewEvent {
                owner: &owner,
                kind: "split",
                year,
                end_year: None,
                phase: EventPhase::CohesionAndSplits,
                causes: vec![],
                participants: vec![
                    ObjectRef::Entity(id.clone()),
                    ObjectRef::Entity(self.groups[new_group].entity.id.clone()),
                ],
                provenance,
            })?
            .encode();
        self.open_chain(
            new_group,
            EventPhase::CohesionAndSplits,
            vec![split.clone()],
        )?;
        self.plan_offices(new_group)?;
        self.sort_offices();
        // The head stays with the group whose charge he holds.
        let heads: BTreeSet<u32> = self.rt[group]
            .required_offices
            .iter()
            .filter_map(|o| self.open_tenure[*o])
            .filter_map(|t| self.tenures[t].person_slot)
            .collect();
        let movers = order.iter().filter(|(_, slot, _)| !heads.contains(slot));
        for (_, slot, m) in movers.take(moved_count) {
            let m = *m as usize;
            self.rt[group].open.retain(|o| *o as usize != m);
            let (predicate, cost) = {
                let record = &self.memberships[m];
                (record.predicate, record.time_cost_bps)
            };
            self.end_membership(m, year, MembershipEnd::Split, Some(split.clone()));
            self.join(new_group, *slot, year, predicate, cost, Err(split.clone()));
        }
        self.groups[group].target_members = self.groups[group]
            .target_members
            .saturating_sub(moved_count as u32);
        let cooldown = year + trigger.cooldown_years;
        self.rt[group].next_split_year = cooldown;
        self.rt[group].below_years = 0;
        self.rt[new_group].next_split_year = cooldown;
        let subject = ObjectRef::Entity(self.groups[new_group].entity.id.clone());
        let object = ObjectRef::Entity(id);
        self.push_relation(
            &owner,
            Predicate::SplitFrom,
            subject,
            object,
            existence,
            Some(split),
        )?;
        self.rebuild_fill_order();
        Ok(())
    }
    fn push_relation(
        &mut self,
        owner: &Owner,
        predicate: Predicate,
        subject: ObjectRef,
        object: ObjectRef,
        valid: TimeSpan,
        cause_event_id: Option<String>,
    ) -> Result<(), HistoryError> {
        let id =
            SemanticAddress::relation(owner, predicate, &subject, &object, valid.start_year, 0)?;
        self.relations.push(SocialRelation {
            id: id.encode(),
            subject,
            predicate,
            object,
            valid,
            role_id: None,
            weight_bps: None,
            time_cost_bps: None,
            cause_event_id,
            end_cause_event_id: None,
            provenance: self.provenance(SOCIAL_RULE),
            rule_id: SOCIAL_RULE.into(),
        });
        Ok(())
    }

    // --- Recruitment (§9.3) ---

    fn join(
        &mut self,
        group: usize,
        slot: u32,
        year: i32,
        predicate: Predicate,
        cost: u16,
        opening: Result<(u32, u32, u32), String>,
    ) {
        let (draw, opening_event_id) = match opening {
            Ok(draw) => (Some(draw), None),
            Err(event) => (None, Some(event)),
        };
        let (vacancy_ordinal, draw_ordinal, candidates) = draw.unwrap_or((0, 0, 0));
        let index = self.memberships.len() as u32;
        self.memberships.push(Membership {
            group: group as u32,
            person_slot: slot,
            predicate,
            opened_year: year,
            valid: TimeSpan {
                start_year: year,
                end_year: self.horizon,
            },
            time_cost_bps: cost,
            vacancy_ordinal,
            draw_ordinal,
            candidates,
            closed_by: None,
            opening_event_id,
            closing_event_id: None,
        });
        self.rt[group].open.push(index);
        if cost > 0 {
            self.committed[slot as usize] += cost;
            self.person_commitments[slot as usize].push(index);
            if self.groups[group].preemptible {
                self.preemptible[slot as usize] += cost;
            }
        }
    }
    fn add_trust(&mut self, group: usize, slot: u32, year: i32) -> Result<(), HistoryError> {
        let others: Vec<u32> = self.rt[group]
            .open
            .iter()
            .map(|m| self.memberships[*m as usize].person_slot)
            .filter(|s| *s != slot)
            .collect();
        if others.is_empty() {
            return Ok(());
        }
        let id = &self.groups[group].entity.id;
        let text = year.to_string();
        let parts = [FRIEND_KEY, id.as_str(), text.as_str()];
        let pick = uniform_below(self.seed, &parts, u64::from(slot), others.len() as u64)? as usize;
        let index = self.links.len();
        self.links.push(PersonLink {
            from_slot: slot,
            to_slot: others[pick],
            predicate: Predicate::Trusts,
            group: group as u32,
            valid: TimeSpan {
                start_year: year,
                end_year: self.horizon,
            },
        });
        self.open_links.push(OpenLink { link: index, group });
        self.links_by_person[slot as usize].push(index as u32);
        self.links_by_person[others[pick] as usize].push(index as u32);
        Ok(())
    }
    /// Fills the open places of a group: filter, score, draw without replacement. A
    /// place without candidate stays open and is reconsidered next year.
    fn fill(&mut self, group: usize, year: i32) -> Result<(), HistoryError> {
        let inputs = self.inputs;
        let record = &self.groups[group];
        if !record.entity.existence.contains(year) {
            return Ok(());
        }
        let target = record.target_members as usize;
        let open = self.rt[group].open.len();
        if open >= target {
            return Ok(());
        }
        let archetype = self.archetype(group);
        let cost = archetype.default_time_cost_bps;
        let filters = &archetype.recruitment_filters;
        let institution = archetype.class == INSTITUTION;
        let site = self.rt[group].site;
        let local = &inputs.catalogue.engine_defaults.local_office;
        let min_age = if institution {
            local.min_age as i32
        } else {
            filters.min_age.map_or(inputs.adult_age, |a| a as i32)
        };
        let max_age = if institution {
            local.max_age as i32
        } else {
            filters.max_age.map_or(i32::MAX, |a| a as i32)
        };
        let barred = match self.jurisdiction_at(site, year).filter(|_| institution) {
            Some(j) => self.ineligible_in(j, year),
            None => Rc::default(),
        };
        let charged = self.charge_conflicts(group, year);
        self.stamp_value += 1;
        let stamp = self.stamp_value;
        for m in &self.rt[group].open {
            self.stamp[self.memberships[*m as usize].person_slot as usize] = stamp;
        }
        let worker = archetype.class == ACTIVITY_UNIT;
        let wanted_sex = filters.sex.as_deref().map(|s| u8::from(s == "female"));
        let table = &self.result.candidate_table;
        let local_reach = [(site, 0)];
        let reach: &[(usize, u64)] = if self.rt[group].wide {
            &self.reach[site]
        } else {
            &local_reach
        };
        let index = (year - self.start) as usize;
        let mut pool: Vec<(u32, u64)> = Vec::new();
        for (s, days) in reach {
            for slot in &self.residents[*s][index] {
                let i = *slot as usize;
                let age = year - table.birth_year[i];
                if age >= min_age
                    && age <= max_age
                    && self.stamp[i] != stamp
                    && wanted_sex.is_none_or(|sex| table.sex[i] == sex)
                    && (!worker || self.workforce[i])
                    && (cost == 0 || self.capacity_at(*slot, year) >= self.committed[i] + cost)
                    && !barred.contains(slot)
                    && !charged.contains(slot)
                {
                    pool.push((*slot, *days));
                }
            }
        }
        if pool.is_empty() {
            return Ok(());
        }
        pool.sort_unstable();
        let founding = self.groups[group].entity.existence.start_year == year;
        if !founding {
            self.count_network(group, year);
        }
        let kin = self.member_families(group);
        let axes = self.groups[group].axes;
        let bonus = self.rt[group].kin_bonus;
        let mut weights: Vec<u64> = pool
            .iter()
            .map(|(slot, days)| {
                let fits = self.fits(group, *slot, *days, &kin);
                let alignment = alignment_fit(self.person_axes[*slot as usize], axes);
                recruitment_weight(&fits, bonus, alignment)
            })
            .collect();
        self.clear_network();
        let id = self.groups[group].entity.id.clone();
        let vacancy = open as u32;
        let count = target - open;
        let draws = draw_without_replacement(self.seed, &id, year, vacancy, &mut weights, count)?;
        let predicate = match archetype.class.as_str() {
            ACTIVITY_UNIT => Predicate::EmployedBy,
            COMMUNITY => Predicate::AffiliatedWith,
            _ => Predicate::MemberOf,
        };
        let circle = archetype.class == FRIEND_CIRCLE;
        for draw in draws {
            let slot = pool[draw.index].0;
            let proof = (vacancy, draw.draw_ordinal, pool.len() as u32);
            self.join(group, slot, year, predicate, cost, Ok(proof));
            if circle {
                self.add_trust(group, slot, year)?;
            }
        }
        Ok(())
    }

    // --- Year ---

    fn rebuild_fill_order(&mut self) {
        let rank = |g: &GroupRecord| match g.entity.kind.as_str() {
            INSTITUTION => 0,
            ACTIVITY_UNIT => 1,
            FRIEND_CIRCLE => 4,
            _ if g.time_cost_bps > 0 => 2,
            _ => 3,
        };
        let groups = &self.groups;
        let mut order: Vec<usize> = (0..groups.len()).collect();
        order.sort_by_key(|i| (rank(&groups[*i]), groups[*i].entity.id.as_str()));
        self.fill_order = order;
    }
    fn sort_offices(&mut self) {
        let offices = &self.offices;
        let mut order: Vec<usize> = (0..offices.len()).collect();
        order.sort_by_key(|i| (offices[*i].priority, offices[*i].office_id.as_str()));
        self.office_order = order;
    }
    fn publish(&mut self, year: i32) -> Result<(), HistoryError> {
        self.stamp_value += 1;
        let stamp = self.stamp_value;
        let mut memberships = 0;
        let mut affiliated = 0;
        for group in 0..self.groups.len() {
            let open = &self.rt[group].open;
            memberships += open.len() as u64;
            let mut published = Vec::with_capacity(open.len());
            let is_job = self.groups[group].entity.kind == ACTIVITY_UNIT;
            let mask = self.rt[group].activity_mask;
            for m in open {
                let slot = self.memberships[*m as usize].person_slot;
                published.push(slot);
                if self.stamp[slot as usize] != stamp {
                    self.stamp[slot as usize] = stamp;
                    affiliated += 1;
                }
                let own = u32::from(self.own_activity[slot as usize]);
                if is_job && own < 32 && mask & (1 << own) != 0 {
                    self.activity_years[slot as usize] += 1;
                }
            }
            published.sort_unstable();
            let rt = &mut self.rt[group];
            rt.published = published;
            let (offices, open_tenure, tenures) = (&self.offices, &self.open_tenure, &self.tenures);
            rt.vacant_published = rt.required_offices.iter().any(|o| {
                offices[*o].valid.contains(year)
                    && open_tenure[*o].is_some_and(|t| tenures[t].mode == TenureMode::Vacant)
            });
        }
        let vacant_offices = self
            .open_tenure
            .iter()
            .flatten()
            .filter(|t| self.tenures[**t].mode == TenureMode::Vacant)
            .count() as u64;
        let mut row = SocialYearBalance {
            year,
            population: 0,
            births: 0,
            deaths: 0,
            incoming: 0,
            outgoing: 0,
            persons_with_affiliation: affiliated,
            memberships,
            vacant_offices,
        };
        for census in self
            .result
            .annual_censuses
            .iter()
            .filter(|c| c.year == year)
        {
            row.population += census.population;
            row.births += census.births;
            row.deaths += census.deaths;
            row.incoming += census.incoming;
            row.outgoing += census.outgoing;
        }
        let residents: u64 = self
            .residents
            .iter()
            .map(|years| years[(year - self.start) as usize].len() as u64)
            .sum();
        let closes = self.balance.last().is_none_or(|p: &SocialYearBalance| {
            p.population + row.births + row.incoming == row.population + row.deaths + row.outgoing
        });
        if residents != row.population || !closes || affiliated > row.population {
            return Err(social_error(
                "social.balance_not_closed",
                &self.domain_id,
                Some(year),
            ));
        }
        self.balance.push(row);
        Ok(())
    }
    fn run(&mut self) -> Result<(), HistoryError> {
        for year in self.start..self.horizon {
            self.ineligible.clear();
            self.close_commitments(year);
            self.close_barred_seats(year);
            for i in 0..self.office_order.len() {
                self.close_office(self.office_order[i], year);
            }
            self.run_cohesion(year)?;
            // Splits move members and add offices: they are filled the same year.
            self.ineligible.clear();
            for i in 0..self.office_order.len() {
                self.assign_office(self.office_order[i], year)?;
            }
            for i in 0..self.fill_order.len() {
                let group = self.fill_order[i];
                self.fill(group, year)?;
            }
            self.publish(year)?;
        }
        Ok(())
    }
    fn plan(&mut self) -> Result<(), HistoryError> {
        let region = self.region;
        for group in &region.groups {
            let Some(site) = self.site_index.get(group.site_id.as_str()).copied() else {
                continue;
            };
            let key = (
                group.site_id.clone(),
                group.entity.archetype_id.clone(),
                group
                    .entity
                    .semantic_address
                    .owned_tail()
                    .get(2)
                    .and_then(|b| b.parse().ok())
                    .unwrap_or(0),
            );
            let next = self.ordinals.entry(key).or_insert(0);
            *next += 1;
            self.adopt(
                group.entity.clone(),
                site,
                None,
                GroupOrigin::RegionPlan,
                None,
            )?;
        }
        let adopted = self.groups.len();
        for site in 0..self.sites.len() {
            self.plan_anchors(site)?;
            self.draw_stock_slots(site)?;
        }
        for group in adopted..self.groups.len() {
            let phase = EventPhase::InstitutionsAndVacancies;
            self.open_chain(group, phase, vec![])?;
        }
        for standing in &region.standings {
            if let (LegalStatus::Proscribed, ObjectRef::Entity(subject), ObjectRef::Entity(j)) =
                (standing.status, &standing.subject, &standing.jurisdiction)
            {
                if let Some(group) = self.groups[..adopted]
                    .iter()
                    .position(|g| &g.entity.id == subject)
                {
                    self.proscribed.push((group, j.clone(), standing.valid));
                }
            }
        }
        for group in 0..self.groups.len() {
            self.plan_offices(group)?;
        }
        self.sort_offices();
        self.rebuild_fill_order();
        Ok(())
    }
    fn site_views(&self) -> Result<BTreeMap<String, SiteViews>, HistoryError> {
        let catalogue = &self.inputs.catalogue;
        let mut views = BTreeMap::new();
        for site in 0..self.sites.len() {
            let family = self.family_of_site(site, self.start)?;
            let overlays = self.overlays(site, self.start)?;
            let adults = self.adults(site, self.start);
            let anchor = self.sites[site];
            let best = |class: &str| {
                catalogue
                    .archetype_definitions
                    .iter()
                    .filter(|a| {
                        a.class == class
                            && a.is_available(
                                adults,
                                &anchor.site_tags,
                                self.inputs.setting,
                                &overlays,
                            )
                    })
                    .max_by_key(|a| {
                        (
                            self.effective_weight(a, &family.common_groups, &overlays),
                            std::cmp::Reverse(a.id.as_str()),
                        )
                    })
                    .map(|a| a.id.clone())
            };
            views.insert(
                anchor.site_id.clone(),
                SiteViews {
                    household_archetype_id: best("household_view"),
                    lineage_archetype_id: best("lineage_view"),
                },
            );
        }
        Ok(views)
    }
    fn person_group_names(
        &self,
        views: &BTreeMap<String, SiteViews>,
    ) -> Result<Vec<PersonGroupName>, HistoryError> {
        // Earliest observed episode fixes each view's naming context. Later moves,
        // members and query windows cannot choose a new surname or culture.
        let table = &self.result.candidate_table;
        let mut origins: BTreeMap<(&str, &str), (i32, usize, u32)> = BTreeMap::new();
        for h in &self.result.household_episodes {
            let Some(&site) = self.site_index.get(h.site_id.as_str()) else {
                continue;
            };
            let year = h.valid.start_year.max(self.start);
            if year >= self.horizon {
                continue;
            }
            let Some(slot) = h
                .member_slots
                .iter()
                .copied()
                .min_by_key(|s| &table.origin_id[*s as usize])
            else {
                continue;
            };
            let candidate = (year, site, slot);
            origins
                .entry(("household", &h.household_id))
                .and_modify(|current| *current = (*current).min(candidate))
                .or_insert(candidate);
        }
        for slot in 0..table.len() {
            if table.source[slot] != SlotSource::Simulated {
                continue;
            }
            for &(site, from, _) in &self.spans[slot] {
                let year = from.max(self.start);
                if year >= self.horizon {
                    continue;
                }
                let candidate = (year, site, slot as u32);
                origins
                    .entry(("lineage", &table.family_key[slot]))
                    .and_modify(|current| *current = (*current).min(candidate))
                    .or_insert(candidate);
            }
        }
        let mut names = Vec::new();
        for ((view_kind, subject), (year, site, slot)) in origins {
            let site_id = &self.sites[site].site_id;
            let Some(view) = views.get(site_id) else {
                continue;
            };
            let archetype_id = if view_kind == "household" {
                &view.household_archetype_id
            } else {
                &view.lineage_archetype_id
            };
            let Some(archetype_id) = archetype_id else {
                continue;
            };
            let definition = &self.inputs.catalogue.archetype_definitions
                [self.archetype_index(archetype_id).expect("view archetype")];
            let surname = &table.family_key[slot as usize];
            let mut refs = BTreeMap::from([
                ("site".into(), site_id.clone()),
                ("region".into(), self.region.region_id.clone()),
                ("surname".into(), surname.clone()),
            ]);
            if let Some(polity) = self.jurisdiction_at(site, year) {
                refs.insert("polity".into(), polity.into());
            }
            // These views have no organizational conduct axes. Ordinary is the
            // neutral naming tone; it asserts no morality about a family.
            let context = NameContext {
                refs,
                activity: Some(self.people[slot as usize].activity_id.clone()),
                tone: Some("ordinary".into()),
            };
            let tree = if let Some(expander) = &self.expander {
                match expander.expand_keyed(
                    &definition.name_profile,
                    &self.people[slot as usize].culture_id,
                    &context,
                    self.world_seed,
                    &format!("{view_kind}|{subject}"),
                ) {
                    Err(e) if e.constraint == "naming.unknown_kind" => None,
                    result => result?,
                }
            } else {
                None
            };
            let fallback = tree.is_none().then(|| FallbackName {
                kind: definition.name_profile.clone(),
                tone: ConductClass::Ordinary,
                site_id: site_id.clone(),
                ordinal: 0,
                provenance: Provenance {
                    status: ProvenanceStatus::Synthetic,
                    source_ref: "name-fallback".into(),
                    rule_id: SOCIAL_RULE.into(),
                },
            });
            names.push(PersonGroupName {
                subject_ref: subject.into(),
                view_kind: view_kind.into(),
                archetype_id: archetype_id.clone(),
                site_id: site_id.clone(),
                surname_ref: surname.clone(),
                name_tree: tree,
                name_fallback: fallback,
            });
        }
        Ok(names)
    }
    fn conflicts(&mut self, region_standings: &[&LegalStanding]) -> Result<(), HistoryError> {
        let by_id: BTreeMap<&str, usize> = self
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| (g.entity.id.as_str(), i))
            .collect();
        let mut pending = Vec::new();
        for standing in region_standings.iter().copied().chain(&self.standings) {
            let ObjectRef::Entity(subject) = &standing.subject else {
                continue;
            };
            let Some(group) = by_id.get(subject.as_str()).map(|g| &self.groups[*g]) else {
                continue;
            };
            if standing.status == LegalStatus::Proscribed && group.visibility == Visibility::Open {
                let owner = Owner::site(&self.inputs.canon_id, &group.site_id);
                let end = standing.valid.end_year.min(group.entity.existence.end_year);
                if end > standing.valid.start_year {
                    let valid = TimeSpan {
                        start_year: standing.valid.start_year,
                        end_year: end,
                    };
                    pending.push((
                        owner,
                        standing.jurisdiction.clone(),
                        standing.subject.clone(),
                        valid,
                        standing.cause_event_id.clone(),
                    ));
                }
            }
        }
        for (owner, subject, object, valid, cause) in pending {
            self.push_relation(
                &owner,
                Predicate::InConflictWith,
                subject,
                object,
                valid,
                cause,
            )?;
        }
        Ok(())
    }
    fn finish(mut self) -> Result<SocialPacket, HistoryError> {
        let region = self.region;
        let inputs = self.inputs;
        let site_views = self.site_views()?;
        let person_group_names = self.person_group_names(&site_views)?;
        let local: BTreeSet<&str> = self.groups.iter().map(|g| g.entity.id.as_str()).collect();
        let region_standings: Vec<&LegalStanding> = region
            .standings
            .iter()
            .filter(|s| match &s.subject {
                ObjectRef::Entity(id) => local.contains(id.as_str()),
                _ => true,
            })
            .collect();
        self.conflicts(&region_standings)?;
        for tenure in self
            .tenures
            .iter()
            .filter(|t| t.mode == TenureMode::Effective)
        {
            let office = self
                .offices
                .iter()
                .find(|o| o.office_id == tenure.office_id)
                .expect("tenure of a known office");
            let owner = Owner::site(&inputs.canon_id, &office.site_id);
            let subject = ObjectRef::PersonSlot(tenure.person_id.clone().unwrap_or_default());
            let object = ObjectRef::Office(tenure.office_id.clone());
            let id = SemanticAddress::relation(
                &owner,
                Predicate::HoldsOffice,
                &subject,
                &object,
                tenure.valid.start_year,
                tenure.tenure_ordinal,
            )?;
            self.relations.push(SocialRelation {
                id: id.encode(),
                subject,
                predicate: Predicate::HoldsOffice,
                object,
                valid: tenure.valid,
                role_id: Some(office.role_key.clone()),
                weight_bps: None,
                time_cost_bps: Some(office.time_cost_bps),
                cause_event_id: tenure.cause_event_id.clone(),
                end_cause_event_id: None,
                provenance: fact_provenance(&inputs.atlas, SOCIAL_RULE),
                rule_id: SOCIAL_RULE.into(),
            });
        }
        // Sorted by ID, with indices rewritten: order of creation is not a fact.
        let mut order: Vec<usize> = (0..self.groups.len()).collect();
        order.sort_by(|a, b| self.groups[*a].entity.id.cmp(&self.groups[*b].entity.id));
        let mut new_index = vec![0u32; order.len()];
        for (new, old) in order.iter().enumerate() {
            new_index[*old] = new as u32;
        }
        let mut groups: Vec<Option<GroupRecord>> = self.groups.drain(..).map(Some).collect();
        let groups: Vec<GroupRecord> = order
            .iter()
            .map(|i| groups[*i].take().expect("each group once"))
            .collect();
        let mut memberships = std::mem::take(&mut self.memberships);
        memberships
            .iter_mut()
            .for_each(|m| m.group = new_index[m.group as usize]);
        memberships.sort_by_key(|m| (m.group, m.valid.start_year, m.person_slot));
        let mut links = std::mem::take(&mut self.links);
        links
            .iter_mut()
            .for_each(|l| l.group = new_index[l.group as usize]);
        links.sort_by_key(|l| (l.group, l.valid.start_year, l.from_slot, l.to_slot));
        let mut cohesion = std::mem::take(&mut self.cohesion);
        cohesion
            .iter_mut()
            .for_each(|c| c.group = new_index[c.group as usize]);
        cohesion.sort_by_key(|c| (c.group, c.year));
        let mut offices = std::mem::take(&mut self.offices);
        offices.sort_by(|a, b| a.office_id.cmp(&b.office_id));
        let mut tenures = std::mem::take(&mut self.tenures);
        tenures.sort_by(|a, b| {
            (&a.office_id, a.tenure_ordinal).cmp(&(&b.office_id, b.tenure_ordinal))
        });
        let mut events = std::mem::take(&mut self.ledger.events);
        events.sort_by(|a, b| a.id.cmp(&b.id));
        let mut standings = std::mem::take(&mut self.standings);
        standings.extend(region_standings.iter().map(|s| (*s).clone()));
        standings.sort_by(|a, b| a.id.cmp(&b.id));
        let mut relations = std::mem::take(&mut self.relations);
        relations.sort_by(|a, b| a.id.cmp(&b.id));
        for group in &groups {
            group.entity.validate()?;
        }
        relations.iter().try_for_each(SocialRelation::validate)?;
        let all_events = [
            inputs.canon_events.as_slice(),
            region.events.as_slice(),
            events.as_slice(),
        ]
        .concat();
        causal_ranks(&all_events)?;
        validate_legal_standings(&standings, &all_events, self.start, self.end)?;
        let table = &self.result.candidate_table;
        let lives = (0..table.len())
            .map(|i| PersonLife {
                birth_year: table.birth_year[i],
                death_year: table.death_year[i],
                source: table.source[i],
            })
            .collect();
        let names = person_names(inputs.world_seed, &self.domain_id, table, &self.people)?;
        let family_surnames = table
            .family_key
            .iter()
            .zip(&names)
            .filter_map(|(key, name)| Some((key.clone(), name.as_ref()?.surname.clone())))
            .collect();
        let thresholds = inputs
            .catalogue
            .engine_defaults
            .alignment
            .conduct_class_thresholds;
        Ok(SocialPacket {
            canon_id: inputs.canon_id.clone(),
            domain_id: self.domain_id.clone(),
            plan_id: self.result.plan_id.clone(),
            plan_fingerprint: self.result.plan_fingerprint.clone(),
            result_fingerprint: self.result.result_fingerprint.clone(),
            rule_id: SOCIAL_RULE.into(),
            split_rule_version: SPLIT_RULE_VERSION.into(),
            timeline: inputs.timeline,
            horizon_end: self.horizon,
            person_thresholds: thresholds.person.into(),
            organization_thresholds: thresholds.organization.into(),
            initial_cohesion_bps: inputs.catalogue.engine_defaults.cohesion_mean_bps as u16,
            people: self.people,
            lives,
            groups,
            site_views,
            person_group_names,
            person_names: names,
            family_surnames,
            memberships,
            links,
            offices,
            tenures,
            relations,
            standings,
            events,
            cohesion,
            balance: self.balance,
            diagnostics: self.diagnostics,
        })
    }
}
/// Control changes carry one `charter` per site; the packet's chains cite it.
fn charter_index(region: &RegionPlan) -> Result<BTreeMap<(String, i32), String>, HistoryError> {
    let mut charters = BTreeMap::new();
    for event in region.events.iter().filter(|e| e.kind == "charter") {
        for participant in &event.participants {
            if let ObjectRef::Site(_) = participant {
                let address = participant.address()?;
                let site_id = address.fields()[1].clone();
                charters.insert((site_id, event.year), event.id.clone());
            }
        }
    }
    Ok(charters)
}
/// Persons declared outlaws by a standing: S0–S5 generate none, an author or S6 may.
fn person_outlaws(
    region: &RegionPlan,
    people: &[PersonAttributes],
) -> Vec<(u32, String, TimeSpan)> {
    let slots: BTreeMap<&str, u32> = people
        .iter()
        .enumerate()
        .map(|(i, p)| (p.person_id.as_str(), i as u32))
        .collect();
    region
        .standings
        .iter()
        .filter_map(|s| match (&s.subject, &s.jurisdiction, s.status) {
            (ObjectRef::PersonSlot(id), ObjectRef::Entity(j), LegalStatus::Proscribed) => {
                Some((*slots.get(id.as_str())?, j.clone(), s.valid))
            }
            _ => None,
        })
        .collect()
}
/// Runs the packet up to `horizon`. A prefix of the same plan gives the same facts
/// as the full packet read before that date (§9.5).
pub(crate) fn build_from_result(
    region: &RegionPlan,
    result: &PopulationResult,
    horizon: i32,
) -> Result<SocialPacket, HistoryError> {
    let mut run = Run::new(region, result, horizon)?;
    run.plan()?;
    run.run()?;
    run.finish()
}
/// Social packet of a domain over the whole horizon (§9.2, §14.1).
pub fn build_social_packet(
    region: &RegionPlan,
    population: &VerifiedPopulation,
) -> Result<SocialPacket, HistoryError> {
    build_from_result(region, population.result(), region.social.timeline.end_year)
}
/// Same packet computed only up to an exclusive year.
pub fn build_social_prefix(
    region: &RegionPlan,
    population: &VerifiedPopulation,
    horizon_end: i32,
) -> Result<SocialPacket, HistoryError> {
    build_from_result(region, population.result(), horizon_end)
}

// --- Readings of a packet: they draw nothing ---

/// Addressed fact of a compact membership (§5). A clipped membership keeps the ID of
/// the full fact: only the interval it shows follows the window.
pub fn membership_relation(
    packet: &SocialPacket,
    membership: &Membership,
) -> Result<SocialRelation, HistoryError> {
    let group = &packet.groups[membership.group as usize];
    let owner = Owner::site(&packet.canon_id, &group.site_id);
    let subject = ObjectRef::PersonSlot(
        packet.people[membership.person_slot as usize]
            .person_id
            .clone(),
    );
    let object = ObjectRef::Entity(group.entity.id.clone());
    let id = SemanticAddress::relation(
        &owner,
        membership.predicate,
        &subject,
        &object,
        membership.opened_year,
        0,
    )?;
    Ok(SocialRelation {
        id: id.encode(),
        subject,
        predicate: membership.predicate,
        object,
        valid: membership.valid,
        role_id: None,
        weight_bps: None,
        time_cost_bps: Some(membership.time_cost_bps),
        cause_event_id: membership.opening_event_id.clone(),
        end_cause_event_id: membership.closing_event_id.clone(),
        provenance: group.entity.provenance.clone(),
        rule_id: SOCIAL_RULE.into(),
    })
}
/// Every affiliation of a person at a date. They overlap freely.
pub fn memberships_of(packet: &SocialPacket, slot: u32, at_year: i32) -> Vec<&Membership> {
    packet
        .memberships
        .iter()
        .filter(|m| m.person_slot == slot && m.valid.contains(at_year))
        .collect()
}
/// Realized members of a group at a date.
pub fn members_at(packet: &SocialPacket, group: u32, at_year: i32) -> Vec<&Membership> {
    let start = packet.memberships.partition_point(|m| m.group < group);
    packet.memberships[start..]
        .iter()
        .take_while(|m| m.group == group)
        .filter(|m| m.valid.contains(at_year))
        .collect()
}
/// Dated state of a group. Expected places and realized members stay distinct.
pub fn group_state(packet: &SocialPacket, group: u32, at_year: i32) -> Option<EntityState> {
    let record = packet.groups.get(group as usize)?;
    if !record.entity.existence.contains(at_year) {
        return None;
    }
    let members = members_at(packet, group, at_year).len() as u64;
    let start = packet.cohesion.partition_point(|c| c.group < group);
    let cohesion = packet.cohesion[start..]
        .iter()
        .take_while(|c| c.group == group && c.year <= at_year)
        .last()
        .map_or(packet.initial_cohesion_bps, |c| c.cohesion_bps);
    Some(EntityState {
        entity_id: record.entity.id.clone(),
        at_year,
        lifecycle: if members == 0 {
            Lifecycle::Dormant
        } else {
            Lifecycle::Active
        },
        member_count: CountMeasure {
            value: Some(members),
            unit: "persons".into(),
            precision: Precision {
                mode: PrecisionMode::Realized,
            },
            coverage: Coverage::Complete,
            basis: "assigned_members".into(),
            at_year,
        },
        cohesion_bps: cohesion,
        legitimacy_bps: 0,
        wealth_index: 0,
        capabilities: Vec::new(),
        alignment: record.axes,
        conduct_tags: record.conduct_tags.clone(),
        visibility: record.visibility,
        goals: Vec::new(),
        assets: Vec::new(),
        footprint_refs: Vec::new(),
        institution_version: SOCIAL_RULE.into(),
    })
}
/// Household of a slot in the state of a year (§25.2).
pub fn household_of(
    population: &VerifiedPopulation,
    slot: u32,
    at_year: i32,
) -> Option<&HouseholdEpisode> {
    population.result().household_episodes.iter().find(|h| {
        let (from, to) = state_span(
            h.valid.start_year,
            h.start_phase,
            h.valid.end_year,
            h.end_phase,
        );
        from <= at_year && at_year < to && h.member_slots.contains(&slot)
    })
}
/// Close family as the table exposes it (§8.4): parents, children and partners. A
/// long lineage is a reading of the simulated core only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FamilyView {
    pub parents: Vec<u32>,
    pub children: Vec<u32>,
    pub partners: Vec<u32>,
    pub lineage_members: Vec<u32>,
}
pub fn family_of(population: &VerifiedPopulation, slot: u32, at_year: i32) -> FamilyView {
    let result = population.result();
    let table = &result.candidate_table;
    let row = slot as usize;
    let partners = result
        .union_allocations
        .iter()
        .filter(|u| u.valid.contains(at_year) && u.partner_slots.contains(&slot))
        .flat_map(|u| u.partner_slots.iter().copied())
        .filter(|s| *s != slot)
        .collect();
    let lineage_members = if table.source[row] == SlotSource::Simulated {
        (0..table.len() as u32)
            .filter(|s| {
                let i = *s as usize;
                table.source[i] == SlotSource::Simulated
                    && table.family_key[i] == table.family_key[row]
            })
            .collect()
    } else {
        Vec::new()
    };
    FamilyView {
        parents: table.mother_slot[row]
            .into_iter()
            .chain(table.father_slot[row])
            .collect(),
        children: result.children_index[row].clone(),
        partners,
        lineage_members,
    }
}
/// Facts of a period, clipped to it: two windows agree on their intersection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SocialWindow {
    pub span: TimeSpan,
    pub tenures: Vec<Tenure>,
    pub memberships: Vec<Membership>,
}
fn clip(valid: TimeSpan, span: TimeSpan) -> Option<TimeSpan> {
    let clipped = TimeSpan {
        start_year: valid.start_year.max(span.start_year),
        end_year: valid.end_year.min(span.end_year),
    };
    (clipped.start_year < clipped.end_year).then_some(clipped)
}
pub fn project_window(packet: &SocialPacket, span: TimeSpan) -> SocialWindow {
    let tenures = packet
        .tenures
        .iter()
        .filter_map(|t| {
            let valid = clip(t.valid, span)?;
            let mut tenure = t.clone();
            if t.valid.end_year >= span.end_year {
                tenure.closed_by = None;
            }
            tenure.valid = valid;
            Some(tenure)
        })
        .collect();
    let memberships = packet
        .memberships
        .iter()
        .filter_map(|m| {
            let valid = clip(m.valid, span)?;
            let mut membership = m.clone();
            if m.valid.end_year >= span.end_year {
                membership.closed_by = None;
                membership.closing_event_id = None;
            }
            membership.valid = valid;
            Some(membership)
        })
        .collect();
    SocialWindow {
        span,
        tenures,
        memberships,
    }
}

/// Report of a packet: groups by class and conduct class, vacancies, mayors (§15.4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SocialYearSummary {
    pub year: i32,
    pub population: u64,
    pub groups_by_class: BTreeMap<String, BTreeMap<ConductClass, u64>>,
    pub dormant_groups: u64,
    pub persons_by_conduct: BTreeMap<ConductClass, u64>,
    pub households: u64,
    pub memberships: u64,
    pub persons_with_affiliation: u64,
    pub vacant_offices: u64,
    pub personal_offices: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SplitSummary {
    pub year: i32,
    pub archetype_id: String,
    pub site_id: String,
    pub moved_members: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SocialSummary {
    pub domain_id: String,
    pub slots: u64,
    pub group_slots_by_class: BTreeMap<String, u64>,
    pub group_slots_by_conduct: BTreeMap<String, BTreeMap<ConductClass, u64>>,
    pub offices: u64,
    pub tenures: u64,
    pub tenure_closures: BTreeMap<TenureEnd, u64>,
    pub vacant_office_years: u64,
    /// Smallest candidate pool any selection saw: how far vacancy is.
    pub fewest_office_candidates: Option<u32>,
    pub lowest_cohesion_bps: Option<u16>,
    pub groups_below_split_threshold: u64,
    pub split_details: Vec<SplitSummary>,
    pub splits: u64,
    pub preemptions: u64,
    pub links: u64,
    pub conflicts: u64,
    pub balance_closed: bool,
    pub years: Vec<SocialYearSummary>,
    pub mayors: BTreeMap<String, Vec<OfficeView>>,
}
pub fn summarize(
    packet: &SocialPacket,
    population: &VerifiedPopulation,
    years: &[i32],
    sites: &[String],
) -> Result<SocialSummary, HistoryError> {
    let thresholds = &packet.organization_thresholds;
    let class_of = |g: &GroupRecord| conduct_class(g.axes, thresholds);
    let mut by_class = BTreeMap::new();
    let mut by_conduct: BTreeMap<String, BTreeMap<ConductClass, u64>> = BTreeMap::new();
    for group in &packet.groups {
        *by_class.entry(group.entity.kind.clone()).or_insert(0) += 1;
        *by_conduct
            .entry(group.entity.kind.clone())
            .or_default()
            .entry(class_of(group))
            .or_insert(0) += 1;
    }
    let mut summaries = Vec::new();
    for year in years {
        let Some(row) = packet.balance.iter().find(|b| b.year == *year) else {
            continue;
        };
        let mut groups_by_class: BTreeMap<String, BTreeMap<ConductClass, u64>> = BTreeMap::new();
        let mut dormant_groups = 0;
        for (i, group) in packet.groups.iter().enumerate() {
            if group.entity.existence.contains(*year) {
                *groups_by_class
                    .entry(group.entity.kind.clone())
                    .or_default()
                    .entry(class_of(group))
                    .or_insert(0) += 1;
                if members_at(packet, i as u32, *year).is_empty() {
                    dormant_groups += 1;
                }
            }
        }
        let mut persons_by_conduct = BTreeMap::new();
        let residents: BTreeSet<u32> = population
            .result()
            .residence_index
            .iter()
            .filter(|r| {
                let (from, to) = state_span(
                    r.valid.start_year,
                    r.start_phase,
                    r.valid.end_year,
                    r.end_phase,
                );
                from <= *year && *year < to
            })
            .map(|r| r.person_slot)
            .collect();
        for slot in &residents {
            let axes = packet.people[*slot as usize].axes;
            *persons_by_conduct
                .entry(conduct_class(axes, &packet.person_thresholds))
                .or_insert(0) += 1;
        }
        let households: BTreeSet<&str> = population
            .result()
            .household_episodes
            .iter()
            .filter(|h| {
                let (from, to) = state_span(
                    h.valid.start_year,
                    h.start_phase,
                    h.valid.end_year,
                    h.end_phase,
                );
                from <= *year && *year < to
            })
            .map(|h| h.household_id.as_str())
            .collect();
        summaries.push(SocialYearSummary {
            year: *year,
            population: row.population,
            groups_by_class,
            dormant_groups,
            persons_by_conduct,
            households: households.len() as u64,
            memberships: row.memberships,
            persons_with_affiliation: row.persons_with_affiliation,
            vacant_offices: row.vacant_offices,
            personal_offices: packet
                .offices
                .iter()
                .filter(|o| !o.collective && o.valid.contains(*year))
                .count() as u64,
        });
    }
    let mut mayors = BTreeMap::new();
    for site in sites {
        if let Some(office) = mayor_office(packet, site) {
            let views = years
                .iter()
                .map(|y| resolve_office(packet, &office.office_id, *y))
                .collect::<Result<_, _>>()?;
            mayors.insert(site.clone(), views);
        }
    }
    let vacant_office_years = packet
        .tenures
        .iter()
        .filter(|t| t.mode == TenureMode::Vacant)
        .map(|t| (t.valid.end_year - t.valid.start_year) as u64)
        .sum();
    let mut tenure_closures = BTreeMap::new();
    for reason in packet.tenures.iter().filter_map(|t| t.closed_by) {
        *tenure_closures.entry(reason).or_insert(0) += 1;
    }
    let fewest_office_candidates = packet
        .tenures
        .iter()
        .map(|t| t.selection_proof.candidates)
        .filter(|c| *c > 0)
        .min();
    let lowest_cohesion_bps = packet.cohesion.iter().map(|c| c.cohesion_bps).min();
    let below: BTreeSet<u32> = packet
        .cohesion
        .iter()
        .filter(|c| c.years_below_threshold > 0)
        .map(|c| c.group)
        .collect();
    let split_details = packet
        .groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.split_from.is_some())
        .map(|(i, g)| SplitSummary {
            year: g.entity.existence.start_year,
            archetype_id: g.entity.archetype_id.clone(),
            site_id: g.site_id.clone(),
            moved_members: members_at(packet, i as u32, g.entity.existence.start_year).len() as u64,
        })
        .collect();
    let kind_count = |kind: &str| packet.events.iter().filter(|e| e.kind == kind).count() as u64;
    Ok(SocialSummary {
        domain_id: packet.domain_id.clone(),
        slots: packet.people.len() as u64,
        group_slots_by_class: by_class,
        group_slots_by_conduct: by_conduct,
        offices: packet.offices.len() as u64,
        tenures: packet.tenures.len() as u64,
        tenure_closures,
        vacant_office_years,
        fewest_office_candidates,
        lowest_cohesion_bps,
        groups_below_split_threshold: below.len() as u64,
        split_details,
        splits: kind_count("split"),
        preemptions: kind_count("employment_preemption"),
        links: packet.links.len() as u64,
        conflicts: packet
            .relations
            .iter()
            .filter(|r| r.predicate == Predicate::InConflictWith)
            .count() as u64,
        balance_closed: true,
        years: summaries,
        mayors,
    })
}
/// `PersonRecord` of §5 for a slot: provider facts and procedural attributes together.
pub fn person_record(
    packet: &SocialPacket,
    population: &VerifiedPopulation,
    slot: u32,
) -> Result<PersonRecord, HistoryError> {
    let result = population.result();
    let table = &result.candidate_table;
    let row = slot as usize;
    let person = &packet.people[row];
    let address = SemanticAddress::decode(&person.person_id)?;
    let parent_refs = table.mother_slot[row]
        .into_iter()
        .chain(table.father_slot[row])
        .map(|parent| ParentRef::Slot {
            person_id: packet.people[parent as usize].person_id.clone(),
        })
        .collect();
    let union_refs = result
        .union_allocations
        .iter()
        .filter(|u| u.partner_slots.contains(&slot))
        .map(|u| local_key(&u.union_id, &packet.domain_id))
        .collect();
    let residence_episodes = population
        .residences_of(slot)
        .iter()
        .map(|r| ResidenceEpisode {
            site_id: r.site_id.clone(),
            valid: r.valid,
        })
        .collect();
    let observed_span = TimeSpan {
        start_year: table.coverage_start_year[row],
        end_year: table.coverage_end_year[row],
    };
    let life = TimeSpan {
        start_year: table.birth_year[row],
        end_year: table.death_year[row].unwrap_or(observed_span.end_year),
    };
    let attributes = BTreeMap::from([
        ("activity".to_owned(), person.activity_id.clone()),
        ("culture".to_owned(), person.culture_id.clone()),
        ("status".to_owned(), person.status_id.clone()),
        ("workforce".to_owned(), person.workforce.to_string()),
    ]);
    let provenance = Provenance {
        status: ProvenanceStatus::Derived,
        source_ref: packet.plan_id.clone(),
        rule_id: ALIGNMENT_SUBSTREAM.into(),
    };
    let lived = life.start_year < life.end_year;
    Ok(PersonRecord {
        entity_id: person.person_id.clone(),
        provider_ref: address.owned_tail()[1].clone(),
        birth_year: table.birth_year[row],
        death_year: table.death_year[row],
        parent_refs,
        union_refs,
        residence_episodes,
        source: match table.source[row] {
            SlotSource::Simulated => PersonSource::LineageSimulated,
            SlotSource::Projected => PersonSource::LineageProjected,
        },
        ancestry_profile: table.ancestry_id[row].clone(),
        social_attribute_episodes: lived
            .then_some(SocialAttributeEpisode {
                valid: life,
                attributes,
            })
            .into_iter()
            .collect(),
        observed_span,
        alignment_episodes: lived
            .then_some(AlignmentEpisode {
                valid: life,
                axes: person.axes,
                conduct_tags: BTreeSet::new(),
                cause_event_id: None,
                provenance,
            })
            .into_iter()
            .collect(),
    })
}
