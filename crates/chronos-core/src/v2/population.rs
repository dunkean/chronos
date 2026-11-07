//! Standalone population plan: schema 1 shape and S0 checks.
use super::atlas::{
    covers, unique_ids, valid_weights, validate_mobility_edge, MobilityEdge, OccupationEpisode,
    OccupationState,
};
use super::canonical::canonical_bytes;
use super::general::{principal_culture, GeneralCanon};
use super::ids::{DomainKey, SemanticAddress};
use super::model::{
    present_optional, required_nullable, ErrorCode, EventPhase, HistoryError, TimeSpan,
};
use super::region::RegionPlan;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    Integer(i64),
    Unsigned(u64),
    Text(String),
    Boolean(bool),
}
impl ParameterValue {
    fn integer(&self) -> Option<i64> {
        match self {
            Self::Integer(n) => Some(*n),
            Self::Unsigned(n) => i64::try_from(*n).ok(),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShockEffects {
    #[serde(
        default,
        deserialize_with = "present_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_mortality_bps: Option<u16>,
    #[serde(
        default,
        deserialize_with = "present_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub mortality_multiplier_ppm: Option<u32>,
    #[serde(
        default,
        deserialize_with = "present_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub fertility_multiplier_ppm: Option<u32>,
    #[serde(
        default,
        deserialize_with = "present_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub migration_multiplier_ppm: Option<u32>,
    #[serde(
        default,
        deserialize_with = "present_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub capacity_multiplier_ppm: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortMode {
    #[serde(rename = "statistical_reservoir")]
    StatisticalReservoir,
    #[serde(rename = "committed_bridge")]
    CommittedBridge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortUnit {
    #[serde(rename = "household")]
    Household,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortRateBasis {
    #[serde(rename = "resident_households_year_opening")]
    ResidentHouseholdsYearOpening,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortRateEpisode {
    pub valid: TimeSpan,
    pub incoming_rate_bps: u16,
    pub outgoing_rate_bps: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryPort {
    pub port_id: String,
    pub contact_id: String,
    pub mode: PortMode,
    pub entry_site_id: String,
    pub external_context_id: String,
    pub external_profile_id: String,
    pub population_expected: u64,
    pub valid: TimeSpan,
    pub unit: PortUnit,
    pub rate_basis: PortRateBasis,
    pub rate_period_years: u32,
    pub rate_episodes: Vec<PortRateEpisode>,
    #[serde(deserialize_with = "required_nullable")]
    pub bridge_plan_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderMode {
    #[serde(rename = "lineage_hybrid")]
    LineageHybrid,
    #[serde(rename = "procedural")]
    Procedural,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationProvider {
    pub mode: ProviderMode,
    pub algorithm_id: String,
    pub runtime_fingerprint: String,
    pub compute_threads: u32,
    pub projection_version: String,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueFingerprints {
    pub history: String,
    pub cultures: String,
    pub demography: String,
    pub institutions: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocietySchema {
    #[serde(rename = "chronos-lineage-society-1")]
    ChronosLineageSociety1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocietyProfile {
    pub id: String,
    pub schema_id: SocietySchema,
    pub parameters: BTreeMap<String, ParameterValue>,
    pub status_weights: BTreeMap<String, u64>,
    pub core_status_weights: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductionMode {
    #[serde(rename = "biparental")]
    Biparental,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AncestryProfile {
    pub id: String,
    pub reproduction_mode: ReproductionMode,
    pub parameters: BTreeMap<String, ParameterValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialClass {
    pub id: String,
    pub provider_rank: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalProfile {
    pub id: String,
    pub ancestry_weights: BTreeMap<String, u64>,
    pub adult_age_min: u64,
    pub adult_age_max: u64,
    pub household_size_weights: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KinshipMode {
    #[serde(rename = "bilateral_projected_v1")]
    BilateralProjectedV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HouseholdModel {
    #[serde(rename = "resident_partition_v1")]
    ResidentPartitionV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KinshipProfile {
    pub id: String,
    pub mode: KinshipMode,
    pub dependent_age: u64,
    pub min_parent_age: u64,
    pub household_model: HouseholdModel,
    pub long_lineages_core_only: bool,
    pub founder_foster_children_limit: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedProfiles {
    pub society: SocietyProfile,
    pub ancestries: Vec<AncestryProfile>,
    pub social_classes: Vec<SocialClass>,
    pub external: Vec<ExternalProfile>,
    pub kinship: Vec<KinshipProfile>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedSettlement {
    pub site_id: String,
    pub provider_local_id: u64,
    pub coordinate_frame: String,
    pub x_mm: i64,
    pub y_mm: i64,
    pub foundation_year: i32,
    pub initial_population: u64,
    pub ancestry_weights: BTreeMap<String, u64>,
    pub history_culture_id: String,
    pub kinship_profile_id: String,
    pub occupation_episodes: Vec<OccupationEpisode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineMarker {
    pub site_id: String,
    pub year: i32,
    pub population: u64,
    pub tolerance_bps: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShockComposition {
    #[serde(rename = "independent_survival_v1")]
    IndependentSurvivalV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationShock {
    /// Identity of the local episode. Several shocks share one source event.
    pub shock_id: String,
    pub source_event_id: String,
    pub valid: TimeSpan,
    pub site_ids: Vec<String>,
    pub severity_profile_id: String,
    pub recovery_years: u64,
    pub composition: ShockComposition,
    pub effects: ShockEffects,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovePrecision {
    #[serde(rename = "expected")]
    Expected,
    #[serde(rename = "exact")]
    Exact,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum HistoricalOperation {
    #[serde(rename = "move_households")]
    MoveHouseholds {
        op_id: String,
        source_event_id: String,
        year: i32,
        origin_site_id: String,
        #[serde(deserialize_with = "required_nullable")]
        destination_site_id: Option<String>,
        #[serde(deserialize_with = "required_nullable")]
        destination_port_id: Option<String>,
        target_households: u64,
        precision: MovePrecision,
    },
    #[serde(rename = "protect_person")]
    ProtectPerson {
        op_id: String,
        source_event_id: String,
        person_id: String,
        valid: TimeSpan,
    },
    #[serde(rename = "force_death")]
    ForceDeath {
        op_id: String,
        source_event_id: String,
        person_id: String,
        year: i32,
    },
    #[serde(rename = "admit_people")]
    AdmitPeople {
        op_id: String,
        source_event_id: String,
        person_ids: Vec<String>,
        source_port_id: String,
        year: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RolePolicy {
    #[serde(rename = "best_effort")]
    BestEffort,
    #[serde(rename = "required")]
    Required,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleDemand {
    pub slot_id: String,
    pub office_id: String,
    pub site_id: String,
    pub valid: TimeSpan,
    pub policy: RolePolicy,
    pub expected_simultaneous_holders: u64,
    pub min_age: u64,
    pub max_age: u64,
    pub status_ids: Vec<String>,
    pub activity_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sex {
    #[serde(rename = "F")]
    F,
    #[serde(rename = "M")]
    M,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedPerson {
    pub person_id: String,
    pub birth_year: i32,
    #[serde(deserialize_with = "required_nullable")]
    pub death_year: Option<i32>,
    pub ancestry_profile_id: String,
    pub sex: Sex,
    pub parent_ids: Vec<String>,
    pub initial_site_id: String,
    pub protected_spans: Vec<TimeSpan>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExactConstraintKind {
    #[serde(rename = "additional_deaths")]
    AdditionalDeaths,
    #[serde(rename = "population_at_date")]
    PopulationAtDate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExactEffectMode {
    #[serde(rename = "additional")]
    Additional,
    #[serde(rename = "replace")]
    Replace,
    #[serde(rename = "assert")]
    Assert,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountBasis {
    #[serde(rename = "year_opening_population")]
    YearOpeningPopulation,
    #[serde(rename = "eligible_survivors")]
    EligibleSurvivors,
    #[serde(rename = "state_at_date")]
    StateAtDate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactConstraint {
    pub constraint_id: String,
    pub source_event_id: String,
    pub kind: ExactConstraintKind,
    pub year: i32,
    pub site_ids: Vec<String>,
    pub count: u64,
    pub phase: u32,
    pub effect_mode: ExactEffectMode,
    pub count_basis: CountBasis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationLimits {
    pub core_target_share_bps: u64,
    pub core_max_share_bps: u64,
    pub max_core_living: u64,
    pub max_core_historical: u64,
    pub max_cohorts: u64,
    pub max_candidate_slots: u64,
    pub max_halo_contacts: u64,
    pub max_attempts: u64,
    pub max_projection_ancestors: u64,
    pub max_projection_descendants: u64,
    pub max_result_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegulationMode {
    #[serde(rename = "baseline_rebased_v1")]
    BaselineRebasedV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regulation {
    pub mode: RegulationMode,
    pub min_fertility_multiplier_ppm: u64,
    pub max_fertility_multiplier_ppm: u64,
    pub max_birth_probability_bps: u16,
    pub response_years: u64,
    pub paused_during_shock_and_recovery: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorePolicy {
    #[serde(rename = "free_try3_then_projected")]
    FreeTry3ThenProjected,
    #[serde(rename = "required_core")]
    RequiredCore,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationPlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub canon_id: String,
    pub domain_id: String,
    pub plan_fingerprint: String,
    pub timeline: TimeSpan,
    pub world_seed: String,
    pub seed: String,
    pub provider: PopulationProvider,
    pub catalogue_fingerprints: CatalogueFingerprints,
    pub demography_society_id: String,
    pub resolved_profiles: ResolvedProfiles,
    pub settlements: Vec<PlannedSettlement>,
    pub baseline_markers: Vec<BaselineMarker>,
    pub shocks: Vec<PopulationShock>,
    pub historical_program: Vec<HistoricalOperation>,
    pub role_demands: Vec<RoleDemand>,
    pub fixed_people: Vec<FixedPerson>,
    pub exact_constraints: Vec<ExactConstraint>,
    pub boundary_ports: Vec<BoundaryPort>,
    pub limits: PopulationLimits,
    pub regulation: Regulation,
    pub core_policy: CorePolicy,
    pub mobility_edges: Vec<MobilityEdge>,
}

impl HistoricalOperation {
    /// Application and normalization key of the program (§24.2): year, phase of
    /// §7.3, `op_id`. The phase follows from the operation type.
    pub fn application_key(&self) -> (i32, EventPhase, &str) {
        match self {
            Self::ProtectPerson { op_id, valid, .. } => {
                (valid.start_year, EventPhase::ContextAndShocks, op_id)
            }
            Self::ForceDeath { op_id, year, .. } => (*year, EventPhase::Deaths, op_id),
            Self::MoveHouseholds { op_id, year, .. } | Self::AdmitPeople { op_id, year, .. } => {
                (*year, EventPhase::Residences, op_id)
            }
        }
    }
}

/// Canonical bytes of the plan for `plan_fingerprint` (§4.2): without `plan_id` or
/// `plan_fingerprint`, lists sorted by ID. Occupation episodes, rate episodes and
/// protected periods are ordered lists and keep their order.
/// Two plans that differ only in the order of their lists have the same bytes.
pub fn plan_canonical_bytes(plan: &PopulationPlan) -> Result<Vec<u8>, HistoryError> {
    let mut p = plan.clone();
    p.provider.capabilities.sort();
    let profiles = &mut p.resolved_profiles;
    profiles.ancestries.sort_by(|a, b| a.id.cmp(&b.id));
    profiles.social_classes.sort_by(|a, b| a.id.cmp(&b.id));
    profiles.external.sort_by(|a, b| a.id.cmp(&b.id));
    profiles.kinship.sort_by(|a, b| a.id.cmp(&b.id));
    p.settlements.sort_by(|a, b| a.site_id.cmp(&b.site_id));
    p.baseline_markers
        .sort_by(|a, b| (&a.site_id, a.year).cmp(&(&b.site_id, b.year)));
    for shock in &mut p.shocks {
        shock.site_ids.sort();
    }
    p.shocks.sort_by(|a, b| a.shock_id.cmp(&b.shock_id));
    for op in &mut p.historical_program {
        if let HistoricalOperation::AdmitPeople { person_ids, .. } = op {
            person_ids.sort();
        }
    }
    p.historical_program
        .sort_by(|a, b| a.application_key().cmp(&b.application_key()));
    for role in &mut p.role_demands {
        role.status_ids.sort();
        role.activity_ids.sort();
    }
    p.role_demands.sort_by(|a, b| a.slot_id.cmp(&b.slot_id));
    for person in &mut p.fixed_people {
        person.parent_ids.sort();
    }
    p.fixed_people.sort_by(|a, b| a.person_id.cmp(&b.person_id));
    for constraint in &mut p.exact_constraints {
        constraint.site_ids.sort();
    }
    p.exact_constraints
        .sort_by(|a, b| a.constraint_id.cmp(&b.constraint_id));
    p.boundary_ports.sort_by(|a, b| a.port_id.cmp(&b.port_id));
    p.mobility_edges.sort_by(|a, b| a.edge_id.cmp(&b.edge_id));
    let mut value = serde_json::to_value(&p).map_err(|e| invalid(e.to_string()))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| invalid("plan.not_an_object"))?;
    object.remove("plan_id");
    object.remove("plan_fingerprint");
    canonical_bytes(&value)
}

/// Resolution of a source variant (`mu_culture_id`, §26.3) by the S1 binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CultureVariant {
    pub history_family_id: String,
    pub demography_society_id: String,
}

/// Canon and pack references already checked by the host. No implicit access
/// to a file, worker or neighbouring repository during validation.
#[derive(Clone, Debug)]
pub struct PlanReferences {
    pub canon_id: String,
    /// World horizon: a plan covers exactly this period (§3.2).
    pub timeline: TimeSpan,
    pub world_seed: u64,
    pub catalogue_fingerprints: CatalogueFingerprints,
    pub event_years: BTreeMap<String, i32>,
    pub office_ids: BTreeSet<String>,
    /// Expected partition, fixed at bootstrap (§3.1): `domain_id` -> sites.
    pub domain_sites: BTreeMap<String, BTreeSet<String>>,
    /// `mu_culture_id` -> family and social profile resolved by S1.
    pub culture_variants: BTreeMap<String, CultureVariant>,
    pub activity_ids: BTreeSet<String>,
    pub contacts: BTreeMap<String, super::atlas::ExternalContact>,
    /// Bounds of the frozen atlas over the whole horizon, not only the population
    /// observed in the query or at the markers.
    pub site_population_expected_max: BTreeMap<String, u64>,
}

/// Validates an ordinary S0–S5 data contract, without announcing that its
/// provider is executed by S0. S6–S8 extensions are rejected.
pub fn validate_plan(plan: &PopulationPlan, refs: &PlanReferences) -> Result<(), HistoryError> {
    validate_plan_inner(plan, refs).map_err(|mut e| {
        e.owner = Some(plan.domain_id.clone());
        e.plan_id = Some(plan.plan_id.clone());
        if e.at_year.is_none() {
            e.at_year = Some(plan.timeline.start_year);
        }
        e
    })
}
fn invalid(message: impl Into<String>) -> HistoryError {
    HistoryError::new(ErrorCode::InvalidPopulationPlan, message)
}
fn unsupported(message: &str) -> HistoryError {
    HistoryError::new(ErrorCode::UnsupportedCapability, message)
}
/// The normative schema is embedded at compile time: no runtime I/O. Shape
/// checks thus stay identical to the published JSON contract.
fn plan_schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/chronos-social-v2/population-plan.schema.json"
        ))
        .expect("shipped schema is valid")
    })
}
/// `plan_id` and `domain_id` are addresses: the plan cannot declare itself
/// to belong to another world, another profile or another domain than its fields.
fn check_plan_identity(p: &PopulationPlan, refs: &PlanReferences) -> Result<(), HistoryError> {
    if p.canon_id != refs.canon_id || p.timeline != refs.timeline {
        return Err(invalid("plan.foreign_canon_or_horizon"));
    }
    if decimal_seed(&p.world_seed)? != refs.world_seed {
        return Err(invalid("plan.world_seed_differs"));
    }
    let domain =
        DomainKey::from_id(&p.domain_id).map_err(|_| invalid("plan.domain_id_not_addressed"))?;
    if domain.world() != p.canon_id || domain.society() != p.demography_society_id {
        return Err(invalid("plan.domain_foreign"));
    }
    if SemanticAddress::plan(&domain)?.encode() != p.plan_id {
        return Err(invalid("plan.plan_id_differs"));
    }
    Ok(())
}
/// A single model is delivered; any future variant must be ruled on here.
fn household_model_accepts(model: HouseholdModel, residence: &str) -> bool {
    match model {
        HouseholdModel::ResidentPartitionV1 => RESIDENCE_MODES.contains(&residence),
    }
}
/// Inline kinship compatible with the social profile (§25.4, §26.3): same
/// majority age, admitted household model, parent never yet dependent.
fn check_kinship(profiles: &ResolvedProfiles) -> Result<(), HistoryError> {
    let parameters = &profiles.society.parameters;
    let majority = parameters["dependent_age"].integer();
    let residence = match &parameters["residence"] {
        ParameterValue::Text(residence) => residence.as_str(),
        _ => return Err(invalid("plan.residence_mistyped")),
    };
    for kinship in &profiles.kinship {
        if i64::try_from(kinship.dependent_age).ok() != majority
            || kinship.min_parent_age < kinship.dependent_age
            || !household_model_accepts(kinship.household_model, residence)
        {
            return Err(invalid("plan.kinship_incompatible"));
        }
    }
    Ok(())
}
fn validate_plan_inner(p: &PopulationPlan, refs: &PlanReferences) -> Result<(), HistoryError> {
    let value = serde_json::to_value(p).map_err(|e| invalid(e.to_string()))?;
    check_shape(&value, plan_schema(), plan_schema(), "plan")?;
    p.timeline.validate(false)?;
    check_plan_identity(p, refs)?;
    let world_seed = refs.world_seed;
    if decimal_seed(&p.seed)?
        != super::ids::random_semantics_v2::domain_seed(
            world_seed,
            &p.domain_id,
            &p.provider.algorithm_id,
        )
    {
        return Err(invalid("plan.domain_seed_inconsistent"));
    }
    if p.catalogue_fingerprints != refs.catalogue_fingerprints {
        return Err(invalid("plan.catalogue_fingerprints_differ"));
    }
    if p.provider.mode != ProviderMode::LineageHybrid {
        return Err(unsupported("plan.unsupported_procedural_provider"));
    }
    let capabilities: BTreeSet<_> = p.provider.capabilities.iter().map(String::as_str).collect();
    if capabilities.iter().any(|c| {
        ![
            "historical_soft_v1",
            "candidate_slots_v1",
            "statistical_reservoir_v1",
        ]
        .contains(c)
    }) {
        return Err(unsupported("plan.unsupported_capability"));
    }
    if !capabilities.contains("historical_soft_v1")
        || (!p.role_demands.is_empty() && !capabilities.contains("candidate_slots_v1"))
        || (!p.boundary_ports.is_empty() && !capabilities.contains("statistical_reservoir_v1"))
    {
        return Err(unsupported("plan.required_capability_missing"));
    }
    if !p.fixed_people.is_empty() || !p.exact_constraints.is_empty() {
        return Err(unsupported("plan.unsupported_fixed_people"));
    }
    if p.core_policy != CorePolicy::FreeTry3ThenProjected {
        return Err(unsupported("plan.unsupported_required_core"));
    }
    let limits = &p.limits;
    if limits.core_target_share_bps > limits.core_max_share_bps
        || limits.max_core_living > limits.max_core_historical
    {
        return Err(invalid("plan.core_caps_inconsistent"));
    }
    if limits.max_candidate_slots > 250_000
        || limits.max_halo_contacts > 16
        || limits.max_result_bytes > 536_870_912
        || limits.max_attempts != 4
        || p.boundary_ports.len() as u64 > limits.max_halo_contacts
    {
        return Err(HistoryError::new(
            ErrorCode::BudgetExceeded,
            "plan.budget_exceeded",
        ));
    }
    let profiles = &p.resolved_profiles;
    if profiles.society.id != p.demography_society_id {
        return Err(invalid("plan.social_profile_inconsistent"));
    }
    for ids in [
        profiles
            .ancestries
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>(),
        profiles
            .social_classes
            .iter()
            .map(|a| a.id.as_str())
            .collect(),
        profiles.external.iter().map(|a| a.id.as_str()).collect(),
        profiles.kinship.iter().map(|a| a.id.as_str()).collect(),
    ] {
        if !unique_ids(ids.into_iter()) {
            return Err(invalid("plan.duplicate_profile_id"));
        }
    }
    let ancestry_ids: BTreeSet<_> = profiles.ancestries.iter().map(|a| a.id.as_str()).collect();
    let status_ids: BTreeSet<_> = profiles
        .social_classes
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    let ranks: BTreeSet<_> = profiles
        .social_classes
        .iter()
        .map(|a| a.provider_rank)
        .collect();
    if ranks != (0..profiles.social_classes.len() as u64).collect() {
        return Err(invalid("plan.social_ranks_invalid"));
    }
    check_weight_refs(&profiles.society.status_weights, &status_ids)?;
    check_weight_refs(&profiles.society.core_status_weights, &status_ids)?;
    check_parameters(&profiles.society.parameters, true)?;
    check_kinship(profiles)?;
    for ancestry in &profiles.ancestries {
        if ancestry.reproduction_mode != ReproductionMode::Biparental {
            return Err(unsupported("plan.unsupported_custom_reproduction"));
        }
        check_parameters(&ancestry.parameters, false)?;
    }
    for profile in &profiles.external {
        check_weight_refs(&profile.ancestry_weights, &ancestry_ids)?;
        positive_weights(&profile.household_size_weights)?;
        if profile.adult_age_min > profile.adult_age_max
            || profile.household_size_weights.keys().any(|s| {
                s.parse::<u32>()
                    .ok()
                    .is_none_or(|n| n == 0 || n.to_string() != *s)
            })
        {
            return Err(invalid("plan.outer_profile_invalid"));
        }
    }
    if !unique_ids(p.settlements.iter().map(|s| s.site_id.as_str()))
        || !unique_ids(p.boundary_ports.iter().map(|s| s.port_id.as_str()))
        || !unique_ids(p.role_demands.iter().map(|s| s.slot_id.as_str()))
        || !unique_ids(p.mobility_edges.iter().map(|s| s.edge_id.as_str()))
    {
        return Err(invalid("plan.duplicate_identities"));
    }
    let local_ids: BTreeSet<_> = p.settlements.iter().map(|s| s.provider_local_id).collect();
    if local_ids.len() != p.settlements.len() {
        return Err(invalid("plan.duplicate_provider_local_id"));
    }
    let sites: BTreeSet<_> = p.settlements.iter().map(|s| s.site_id.as_str()).collect();
    let expected = refs
        .domain_sites
        .get(&p.domain_id)
        .ok_or_else(|| invalid("plan.domain_not_in_partition"))?;
    if !sites
        .iter()
        .copied()
        .eq(expected.iter().map(String::as_str))
    {
        return Err(invalid("plan.sites_differ_from_partition"));
    }
    let frames: BTreeSet<_> = p.settlements.iter().map(|s| &s.coordinate_frame).collect();
    if frames.len() != 1 {
        return Err(invalid("plan.multiple_frames"));
    }
    let mut max_population: BTreeMap<_, _> = p
        .settlements
        .iter()
        .map(|s| (s.site_id.as_str(), s.initial_population))
        .collect();
    let culture_variant = |variant: &str| {
        refs.culture_variants
            .get(variant)
            .ok_or_else(|| invalid("plan.culture_variant_unbound"))
    };
    // Mixed profiles in a domain: an S8 capability, never a redistribution.
    let same_society = |variant: &CultureVariant| {
        if variant.demography_society_id == p.demography_society_id {
            Ok(())
        } else {
            Err(unsupported("plan.unsupported_mixed_profiles"))
        }
    };
    let activities: BTreeSet<_> = refs.activity_ids.iter().map(String::as_str).collect();
    for s in &p.settlements {
        check_weight_refs(&s.ancestry_weights, &ancestry_ids)?;
        if s.foundation_year > p.timeline.start_year
            || s.occupation_episodes
                .iter()
                .any(|e| e.state == OccupationState::Abandoned)
        {
            return Err(unsupported("plan.unsupported_dynamic_sites"));
        }
        if !covers(s.occupation_episodes.iter().map(|e| e.valid), p.timeline) {
            return Err(invalid("plan.occupation_episodes_invalid"));
        }
        if !profiles
            .kinship
            .iter()
            .any(|k| k.id == s.kinship_profile_id)
        {
            return Err(invalid("plan.kinship_profile_missing"));
        }
        let (family, variant) = s
            .history_culture_id
            .split_once('/')
            .filter(|(family, variant)| !family.is_empty() && !variant.contains('/'))
            .ok_or_else(|| invalid("plan.history_culture_outside_family"))?;
        let principal = culture_variant(variant)?;
        if principal.history_family_id != family {
            return Err(invalid("plan.main_variant_outside_family"));
        }
        same_society(principal)?;
        for e in &s.occupation_episodes {
            if !valid_weights(&e.culture_weights)
                || !valid_weights(&e.activity_weights)
                || e.activity_weights
                    .keys()
                    .any(|k| !activities.contains(k.as_str()))
            {
                return Err(invalid("plan.occupation_weights_invalid"));
            }
            for (variant, weight) in &e.culture_weights {
                let resolved = culture_variant(variant)?;
                if *weight > 0 {
                    same_society(resolved)?;
                }
            }
        }
    }
    let mut marker_dates = BTreeSet::new();
    for m in &p.baseline_markers {
        if !sites.contains(m.site_id.as_str())
            || !p.timeline.contains(m.year)
            || !marker_dates.insert((&m.site_id, m.year))
        {
            return Err(invalid("plan.marker_invalid"));
        }
        let max = max_population
            .get_mut(m.site_id.as_str())
            .expect("site was checked");
        *max = (*max).max(m.population);
    }
    let mut domain_max = 0u128;
    for (site, observed_max) in max_population {
        let bound = refs
            .site_population_expected_max
            .get(site)
            .ok_or_else(|| invalid("plan.atlas_bound_missing"))?;
        if observed_max > *bound {
            return Err(invalid("plan.population_over_atlas_bound"));
        }
        domain_max += u128::from(*bound);
    }
    if domain_max > 20_000 {
        return Err(HistoryError::new(
            ErrorCode::BudgetExceeded,
            "plan.population_over_cap",
        ));
    }
    let event = |id: &str, year: i32| -> Result<(), HistoryError> {
        if refs.event_years.get(id).is_none_or(|y| *y > year) {
            Err(invalid("plan.source_event_invalid"))
        } else {
            Ok(())
        }
    };
    let span = |s: TimeSpan| -> Result<(), HistoryError> {
        s.validate(false)?;
        if !p.timeline.encloses(s) {
            Err(invalid("plan.interval_outside_plan"))
        } else {
            Ok(())
        }
    };
    // One exposure per event and village, whatever its dates (§7.5).
    let mut exposures = BTreeSet::new();
    if !unique_ids(p.shocks.iter().map(|s| s.shock_id.as_str())) {
        return Err(invalid("plan.shock_id_invalid_or_duplicated"));
    }
    for s in &p.shocks {
        span(s.valid)?;
        event(&s.source_event_id, s.valid.start_year)?;
        if s.shock_id == s.source_event_id {
            return Err(invalid("plan.shock_id_equals_source_event"));
        }
        for id in &s.site_ids {
            if !sites.contains(id.as_str()) || !exposures.insert((&s.source_event_id, id)) {
                return Err(invalid("plan.shock_exposure_duplicated_or_site_unknown"));
            }
        }
    }
    let mut ops = BTreeSet::new();
    for op in &p.historical_program {
        match op {
            HistoricalOperation::MoveHouseholds {
                op_id,
                source_event_id,
                year,
                origin_site_id,
                destination_site_id,
                destination_port_id,
                precision,
                ..
            } => {
                if *precision == MovePrecision::Exact {
                    return Err(unsupported("plan.unsupported_exact_movement"));
                }
                event(source_event_id, *year)?;
                if !ops.insert(op_id)
                    || !p.timeline.contains(*year)
                    || !sites.contains(origin_site_id.as_str())
                    || destination_site_id.is_some() == destination_port_id.is_some()
                {
                    return Err(invalid("plan.movement_invalid"));
                }
                if let Some(dest) = destination_site_id {
                    if dest == origin_site_id || !sites.contains(dest.as_str()) {
                        return Err(invalid("plan.movement_destination_invalid"));
                    }
                }
                if let Some(port) = destination_port_id {
                    if !p.boundary_ports.iter().any(|p| {
                        &p.port_id == port
                            && p.valid.contains(*year)
                            && &p.entry_site_id == origin_site_id
                    }) {
                        return Err(invalid("plan.port_destination_invalid"));
                    }
                }
            }
            _ => return Err(unsupported("plan.unsupported_personal_operation")),
        }
    }
    for r in &p.role_demands {
        if r.policy != RolePolicy::BestEffort {
            return Err(unsupported("plan.unsupported_required_role"));
        }
        span(r.valid)?;
        if !sites.contains(r.site_id.as_str())
            || !refs.office_ids.contains(&r.office_id)
            || r.min_age > r.max_age
            || r.status_ids
                .iter()
                .any(|id| !status_ids.contains(id.as_str()))
            || r.activity_ids
                .iter()
                .any(|id| !activities.contains(id.as_str()))
        {
            return Err(invalid("plan.role_demand_invalid"));
        }
    }
    for port in &p.boundary_ports {
        if port.mode != PortMode::StatisticalReservoir || port.bridge_plan_id.is_some() {
            return Err(unsupported("plan.unsupported_named_bridge"));
        }
        span(port.valid)?;
        let contact = refs
            .contacts
            .get(&port.contact_id)
            .ok_or_else(|| invalid("plan.contact_missing"))?;
        // Each side of the contact opens its port under the same `contact_id`.
        let side = contact
            .site_end(&port.entry_site_id)
            .ok_or_else(|| invalid("plan.port_foreign_to_contact"))?;
        if !sites.contains(port.entry_site_id.as_str())
            || side.external_context_id != port.external_context_id
            || side.external_profile_id != port.external_profile_id
            || side.population_expected != port.population_expected
            || !contact.valid.encloses(port.valid)
            || !profiles
                .external
                .iter()
                .any(|e| e.id == port.external_profile_id)
            || !covers(port.rate_episodes.iter().map(|e| e.valid), port.valid)
        {
            return Err(invalid("plan.rate_episodes_inconsistent"));
        }
    }
    for edge in &p.mobility_edges {
        validate_mobility_edge(edge, p.timeline, &sites)?;
    }
    Ok(())
}
fn decimal_seed(s: &str) -> Result<u64, HistoryError> {
    let n: u64 = s.parse().map_err(|_| invalid("plan.seed_out_of_range"))?;
    if n.to_string() != s {
        return Err(invalid("plan.decimal_seed_not_normalized"));
    }
    Ok(n)
}
fn positive_weights(weights: &BTreeMap<String, u64>) -> Result<(), HistoryError> {
    if weights.is_empty() || !weights.values().any(|v| *v > 0) {
        Err(invalid("plan.weights_sum_zero"))
    } else {
        Ok(())
    }
}
fn check_weight_refs(
    weights: &BTreeMap<String, u64>,
    ids: &BTreeSet<&str>,
) -> Result<(), HistoryError> {
    positive_weights(weights)?;
    if weights.keys().any(|k| !ids.contains(k.as_str())) {
        Err(invalid("plan.weight_reference_missing"))
    } else {
        Ok(())
    }
}

/// Values of `Society.residence` in Lineage Lab's `config.py`.
const RESIDENCE_MODES: &[&str] = &["patrilocal", "matrilocal", "either"];
const SOCIETY_PARAMETERS: &[&str] = &[
    "childfree_rate_bps",
    "marriage_rate_bps",
    "founder_match_participation_bps",
    "marriage_min_age",
    "marriage_max_age",
    "max_age_gap",
    "divorce_rate_bps",
    "remarriage_delay_years",
    "kinship_depth",
    "matching_attempts",
    "status_affinity_bps",
    "status_inheritance_bps",
    "activity_inheritance_bps",
    "migration_rate_bps",
    "dependent_age",
    "residence",
    "distance_scale_mm",
    "marriage_radius_mm",
    "migration_radius_mm",
    "max_neighbors",
    "local_marriage_weight",
    "capacity_growth_ppm",
];
const ANCESTRY_PARAMETERS: &[&str] = &[
    "max_age",
    "infant_mortality_bps",
    "child_mortality_bps",
    "child_max_age",
    "adult_mortality_bps",
    "aging_coefficient_ppm",
    "aging_exponent_ppm",
    "male_mortality_factor_ppm",
    "maternal_mortality_bps",
    "male_birth_probability_bps",
    "female_infertility_rate_bps",
    "male_infertility_rate_bps",
    "fertility_peak_bps",
    "fertility_peak_age",
    "fertility_width_years",
    "fertility_min_age",
    "fertility_max_age",
    "birth_spacing_years",
];
fn check_parameters(
    parameters: &BTreeMap<String, ParameterValue>,
    society: bool,
) -> Result<(), HistoryError> {
    let allowed = if society {
        SOCIETY_PARAMETERS
    } else {
        ANCESTRY_PARAMETERS
    };
    if parameters.len() != allowed.len() || allowed.iter().any(|k| !parameters.contains_key(*k)) {
        return Err(invalid("plan.profile_parameter_invalid"));
    }
    for (key, value) in parameters {
        if key == "residence" {
            if !matches!(value, ParameterValue::Text(s) if RESIDENCE_MODES.contains(&s.as_str())) {
                return Err(invalid("plan.unknown_residence_mode"));
            }
            continue;
        }
        let n = value
            .integer()
            .ok_or_else(|| invalid("plan.numeric_parameter_mistyped"))?;
        if n < 0
            || (key.ends_with("_bps") && n > 10_000)
            || (key.ends_with("_ppm") && n > 10_000_000)
            || (key.ends_with("_mm") && n > 9_000_000_000_000_000)
        {
            return Err(invalid("plan.parameter_unit_or_bound_invalid"));
        }
    }
    let n = |key: &str| parameters[key].integer().expect("type was checked");
    if society {
        if n("marriage_min_age") > n("marriage_max_age")
            || n("distance_scale_mm") == 0
            || n("matching_attempts") == 0
            || n("max_neighbors") == 0
        {
            return Err(invalid("plan.ages_scale_attempts_inconsistent"));
        }
    } else if n("fertility_min_age") > n("fertility_peak_age")
        || n("fertility_peak_age") > n("fertility_max_age")
        || n("fertility_max_age") > n("max_age")
        || n("child_max_age") > n("max_age")
        || n("fertility_width_years") == 0
        || n("birth_spacing_years") == 0
    {
        return Err(invalid("plan.biological_periods_inconsistent"));
    }
    Ok(())
}

// Private interpreter limited to the keywords present in the shipped schema.
// Rust types already ensure the shape; this pass checks bounds,
// cardinalities and constants, including after direct construction in Rust.
fn check_shape(v: &Value, s: &Value, root: &Value, path: &str) -> Result<(), HistoryError> {
    let fail = || invalid("plan.schema_shape").with_param("path", path);
    if let Some(r) = s.get("$ref").and_then(Value::as_str) {
        return check_shape(
            v,
            root.pointer(&r[1..]).expect("shipped local reference"),
            root,
            path,
        );
    }
    if let Some(c) = s.get("const") {
        if v != c {
            return Err(fail());
        }
    }
    if let Some(e) = s.get("enum").and_then(Value::as_array) {
        if !e.contains(v) {
            return Err(fail());
        }
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(branches) = s.get(keyword).and_then(Value::as_array) {
            let valid = branches
                .iter()
                .filter(|b| check_shape(v, b, root, path).is_ok())
                .count();
            if valid == 0 || (keyword == "oneOf" && valid != 1) {
                return Err(fail());
            }
        }
    }
    if let Some(branches) = s.get("allOf").and_then(Value::as_array) {
        for branch in branches {
            check_shape(v, branch, root, path)?;
        }
    }
    if let Some(condition) = s.get("if") {
        let branch = if check_shape(v, condition, root, path).is_ok() {
            "then"
        } else {
            "else"
        };
        if let Some(b) = s.get(branch) {
            check_shape(v, b, root, path)?;
        }
    }
    if let Some(kind) = s.get("type").and_then(Value::as_str) {
        let ok = match kind {
            "object" => v.is_object(),
            "array" => v.is_array(),
            "string" => v.is_string(),
            "integer" => v.as_number().is_some_and(|n| !n.is_f64()),
            "boolean" => v.is_boolean(),
            "null" => v.is_null(),
            _ => false,
        };
        if !ok {
            return Err(fail());
        }
    }
    if let Some(obj) = v.as_object() {
        if let Some(required) = s.get("required").and_then(Value::as_array) {
            if required
                .iter()
                .any(|k| !obj.contains_key(k.as_str().expect("field name")))
            {
                return Err(fail());
            }
        }
        if s.get("minProperties")
            .and_then(Value::as_u64)
            .is_some_and(|n| obj.len() < n as usize)
        {
            return Err(fail());
        }
        for (key, value) in obj {
            if let Some(field) = s.get("properties").and_then(|p| p.get(key)) {
                check_shape(value, field, root, &format!("{path}.{key}"))?;
            } else if let Some(additional) = s.get("additionalProperties") {
                if additional == &Value::Bool(false) {
                    return Err(fail());
                }
                if additional.is_object() {
                    check_shape(value, additional, root, &format!("{path}.{key}"))?;
                }
            }
        }
    }
    if let Some(array) = v.as_array() {
        if s.get("minItems")
            .and_then(Value::as_u64)
            .is_some_and(|n| array.len() < n as usize)
            || s.get("maxItems")
                .and_then(Value::as_u64)
                .is_some_and(|n| array.len() > n as usize)
        {
            return Err(fail());
        }
        if s.get("uniqueItems") == Some(&Value::Bool(true)) {
            let mut seen = BTreeSet::new();
            for value in array {
                if !seen.insert(super::canonical::canonical_bytes(value)?) {
                    return Err(fail());
                }
            }
        }
        if let Some(item) = s.get("items") {
            for (i, value) in array.iter().enumerate() {
                check_shape(value, item, root, &format!("{path}[{i}]"))?;
            }
        }
    }
    if let Some(string) = v.as_str() {
        let length = string.chars().count();
        if s.get("minLength")
            .and_then(Value::as_u64)
            .is_some_and(|n| length < n as usize)
            || s.get("maxLength")
                .and_then(Value::as_u64)
                .is_some_and(|n| length > n as usize)
        {
            return Err(fail());
        }
        if s.get("pattern").is_some() {
            decimal_seed(string)?;
        }
    }
    if let Some(number) = v.as_number() {
        let integer = number
            .as_i64()
            .map(i128::from)
            .or_else(|| number.as_u64().map(i128::from))
            .ok_or_else(fail)?;
        if s.get("minimum")
            .and_then(Value::as_i64)
            .is_some_and(|n| integer < i128::from(n))
            || s.get("maximum")
                .and_then(Value::as_i64)
                .is_some_and(|n| integer > i128::from(n))
        {
            return Err(fail());
        }
    }
    Ok(())
}

/// `plan_fingerprint` of a compiled plan: SHA-256 stays with the host (§4.2), which
/// replaces this value from `plan_canonical_bytes`.
pub const PLAN_FINGERPRINT_PENDING: &str = "pending-host-sha256";

/// References of a plan, prepared from the canon and its regional plan alone.
pub fn plan_references(
    canon: &GeneralCanon,
    region: &RegionPlan,
) -> Result<PlanReferences, HistoryError> {
    let mut culture_variants = BTreeMap::new();
    for binding in &canon.catalogue.bindings {
        let template = &binding.demography_society_template_id;
        if let Some(society) = canon.config.society_ids.get(template) {
            culture_variants.insert(
                binding.mu_culture_id.clone(),
                CultureVariant {
                    history_family_id: binding.history_family_id.clone(),
                    demography_society_id: society.clone(),
                },
            );
        }
    }
    Ok(PlanReferences {
        canon_id: canon.canon_id.clone(),
        timeline: canon.timeline,
        world_seed: canon.world_seed,
        catalogue_fingerprints: canon.config.catalogue_fingerprints.clone(),
        event_years: canon
            .events
            .iter()
            .chain(&region.events)
            .map(|e| (e.id.clone(), e.year))
            .collect(),
        office_ids: BTreeSet::new(),
        domain_sites: canon
            .domains
            .iter()
            .map(|d| (d.domain_id.clone(), d.site_ids.iter().cloned().collect()))
            .collect(),
        culture_variants,
        activity_ids: canon.catalogue.required_activity_coverage.clone(),
        contacts: canon
            .atlas
            .external_contacts
            .iter()
            .map(|c| (c.contact_id.clone(), c.clone()))
            .collect(),
        site_population_expected_max: canon
            .atlas
            .settlements
            .iter()
            .map(|s| (s.site_id.clone(), s.population_expected_max))
            .collect(),
    })
}
/// Budgets of one domain. The configured absolute core caps are world-wide, so a
/// large domain could exceed them at its first census with its target founders and
/// lose every core attempt (§8.4). They rise to the share ceiling applied to the
/// domain's expected maximum, keeping their living/historical ratio. A cap of zero
/// disables the core and stays zero.
fn domain_limits(limits: &PopulationLimits, expected_max: u64) -> PopulationLimits {
    let ceiling = expected_max * limits.core_max_share_bps / 10_000;
    let mut result = limits.clone();
    if limits.max_core_living > 0 && ceiling > limits.max_core_living {
        result.max_core_historical = limits.max_core_historical * ceiling / limits.max_core_living;
        result.max_core_living = ceiling;
    }
    result
}

/// Compiles the fixed-horizon plan of a domain (§3.2, §14.1) and validates it. Its
/// markers come from the shock-free baseline; shocks arrive only through `shocks`.
pub fn plan_population(
    canon: &GeneralCanon,
    region: &RegionPlan,
    domain_id: &str,
) -> Result<PopulationPlan, HistoryError> {
    let located = |key: &str| HistoryError {
        owner: Some(domain_id.into()),
        at_year: Some(canon.timeline.start_year),
        ..invalid(key)
    };
    let domain = canon
        .domains
        .iter()
        .find(|d| d.domain_id == domain_id)
        .ok_or_else(|| located("plan.unknown_domain"))?;
    if region.canon_id != canon.canon_id || region.region_id != domain.region_id {
        return Err(located("plan.foreign_region_plan"));
    }
    let defaults = &canon.config.population;
    let inside: BTreeSet<_> = domain.site_ids.iter().map(String::as_str).collect();
    let mut settlements = Vec::new();
    let mut expected_max = 0;
    for (local_id, site_id) in domain.site_ids.iter().enumerate() {
        let site = canon
            .atlas
            .settlements
            .iter()
            .find(|s| &s.site_id == site_id)
            .ok_or_else(|| located("plan.unknown_site"))?;
        expected_max += site.population_expected_max;
        let culture =
            principal_culture(&canon.atlas, site_id).ok_or_else(|| located("plan.unknown_site"))?;
        let family = &canon.catalogue.binding(&culture)?.history_family_id;
        settlements.push(PlannedSettlement {
            site_id: site_id.clone(),
            provider_local_id: local_id as u64,
            coordinate_frame: site.coordinate_frame.clone(),
            x_mm: site.x_mm,
            y_mm: site.y_mm,
            foundation_year: site.foundation_year,
            initial_population: site.initial_population,
            ancestry_weights: defaults.ancestry_weights.clone(),
            history_culture_id: format!("{family}/{culture}"),
            kinship_profile_id: defaults.kinship.id.clone(),
            occupation_episodes: site.occupation_episodes.clone(),
        });
    }
    let baseline_markers = domain
        .site_ids
        .iter()
        .flat_map(|site_id| {
            region.site_baselines[site_id]
                .iter()
                .map(move |(year, population)| BaselineMarker {
                    site_id: site_id.clone(),
                    year: *year,
                    population: *population,
                    tolerance_bps: defaults.baseline_tolerance_bps,
                })
        })
        .collect();
    // One shock per local episode, in canonical order: the host hashes this list.
    let mut shocks: Vec<_> = region
        .shocks
        .iter()
        .filter(|s| inside.contains(s.site_id.as_str()))
        .map(|local| PopulationShock {
            shock_id: local.episode_id.clone(),
            source_event_id: local.source_event_id.clone(),
            valid: local.valid,
            site_ids: vec![local.site_id.clone()],
            severity_profile_id: local.severity_profile_id.clone(),
            recovery_years: local.recovery_years,
            composition: ShockComposition::IndependentSurvivalV1,
            effects: local.effects.clone(),
        })
        .collect();
    shocks.sort_by(|a, b| a.shock_id.cmp(&b.shock_id));
    let mut historical_program: Vec<_> = region
        .displacements
        .iter()
        .filter(|d| inside.contains(d.origin_site_id.as_str()))
        .map(|d| HistoricalOperation::MoveHouseholds {
            op_id: d.op_id.clone(),
            source_event_id: d.event_id.clone(),
            year: d.year,
            origin_site_id: d.origin_site_id.clone(),
            destination_site_id: d.destination_site_id.clone(),
            destination_port_id: d.destination_port_id.clone(),
            target_households: d.target_households,
            precision: MovePrecision::Expected,
        })
        .collect();
    historical_program.sort_by(|a, b| a.application_key().cmp(&b.application_key()));
    let template = canon
        .catalogue
        .demography_society_templates
        .iter()
        .find(|t| canon.config.society_ids.get(&t.id) == Some(&domain.demography_society_id))
        .ok_or_else(|| located("plan.society_template_missing"))?;
    let key = DomainKey::from_id(domain_id)?;
    let plan = PopulationPlan {
        schema_version: 1,
        plan_id: SemanticAddress::plan(&key)?.encode(),
        canon_id: canon.canon_id.clone(),
        domain_id: domain_id.into(),
        plan_fingerprint: PLAN_FINGERPRINT_PENDING.into(),
        timeline: canon.timeline,
        world_seed: canon.world_seed.to_string(),
        seed: super::ids::random_semantics_v2::domain_seed(
            canon.world_seed,
            domain_id,
            &canon.config.provider.algorithm_id,
        )
        .to_string(),
        provider: canon.config.provider.clone(),
        catalogue_fingerprints: canon.config.catalogue_fingerprints.clone(),
        demography_society_id: domain.demography_society_id.clone(),
        resolved_profiles: ResolvedProfiles {
            society: template.resolved(&domain.demography_society_id),
            ancestries: defaults.ancestries.clone(),
            social_classes: defaults.social_classes.clone(),
            external: defaults.external.clone(),
            kinship: vec![defaults.kinship.clone()],
        },
        settlements,
        baseline_markers,
        shocks,
        historical_program,
        role_demands: Vec::new(),
        fixed_people: Vec::new(),
        exact_constraints: Vec::new(),
        boundary_ports: region.contexts[domain_id].ports.clone(),
        limits: domain_limits(&defaults.limits, expected_max),
        regulation: defaults.regulation.clone(),
        core_policy: CorePolicy::FreeTry3ThenProjected,
        mobility_edges: canon
            .atlas
            .mobility_edges
            .iter()
            .filter(|e| {
                inside.contains(e.from_site_id.as_str()) && inside.contains(e.to_site_id.as_str())
            })
            .cloned()
            .collect(),
    };
    validate_plan(&plan, &plan_references(canon, region)?)?;
    Ok(plan)
}

/// Format of the complete provider file (§13).
pub const POPULATION_RESULT_FORMAT: &str = "population-result-json-2";
/// Version of the checks `validate_population` runs; recorded in its proof.
pub const POPULATION_VALIDATOR: &str = "chronos-core-population-1";

/// Who produced a slot: the simulated core or the projected background.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotSource {
    Simulated,
    Projected,
}
/// Deaths, exits and coverage ends are different states (§24.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepartureState {
    Dead,
    DepartedExternal,
    ResidentAtCoverageEnd,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginType {
    Founder,
    Birth,
    Arrival,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidenceEnd {
    Death,
    Transfer,
    DepartedExternal,
    CoverageEnd,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HouseholdKind {
    Single,
    Union,
    SyntheticCare,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnionSource {
    Simulated,
    Projected,
    IncomingFormed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreAttributes {
    pub status_rank: u32,
    pub activity_id: String,
    pub observation: String,
}
/// Columns of every slot that resided in the domain (§8.2), sorted by `origin_id`.
/// A person reference elsewhere is a zero-based row here, `null` when unknown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateTable {
    pub origin_id: Vec<String>,
    pub origin_cohort_id: Vec<String>,
    pub sex_ordinal: Vec<u32>,
    pub source: Vec<SlotSource>,
    pub ancestry_id: Vec<String>,
    pub sex: Vec<u8>,
    pub birth_year: Vec<i32>,
    pub death_year: Vec<Option<i32>>,
    pub death_site_id: Vec<Option<String>>,
    pub coverage_start_year: Vec<i32>,
    pub coverage_end_year: Vec<i32>,
    pub departure_state: Vec<DepartureState>,
    pub departure_year: Vec<Option<i32>>,
    pub mother_slot: Vec<Option<u32>>,
    pub father_slot: Vec<Option<u32>>,
    pub family_key: Vec<String>,
    pub reproduction_flags: Vec<u32>,
    pub core_attributes: Vec<Option<CoreAttributes>>,
}
impl CandidateTable {
    pub fn len(&self) -> usize {
        self.origin_id.len()
    }
    pub fn is_empty(&self) -> bool {
        self.origin_id.is_empty()
    }
    fn columns_aligned(&self) -> bool {
        let n = self.len();
        [
            self.origin_cohort_id.len(),
            self.sex_ordinal.len(),
            self.source.len(),
            self.ancestry_id.len(),
            self.sex.len(),
            self.birth_year.len(),
            self.death_year.len(),
            self.death_site_id.len(),
            self.coverage_start_year.len(),
            self.coverage_end_year.len(),
            self.departure_state.len(),
            self.departure_year.len(),
            self.mother_slot.len(),
            self.father_slot.len(),
            self.family_key.len(),
            self.reproduction_flags.len(),
            self.core_attributes.len(),
        ]
        .iter()
        .all(|len| *len == n)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginCohort {
    pub origin_cohort_id: String,
    pub origin_type: OriginType,
    pub ancestry_id: String,
    pub birth_year: i32,
    pub sealed: bool,
    pub sex_counts: [u32; 2],
}
/// Intervals read on `(year, phase)`: `valid` alone would lose same-year visits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotResidence {
    pub person_slot: u32,
    pub site_id: String,
    pub valid: TimeSpan,
    pub start_phase: u8,
    pub end_phase: u8,
    pub end_reason: ResidenceEnd,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdEpisode {
    pub household_id: String,
    pub kind: HouseholdKind,
    pub formation_reason: String,
    pub site_id: String,
    pub member_slots: Vec<u32>,
    pub valid: TimeSpan,
    pub start_phase: u8,
    pub end_phase: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnionAllocation {
    pub union_id: String,
    pub family_key: String,
    pub partner_slots: Vec<u32>,
    pub site_at_formation: String,
    pub source: UnionSource,
    pub valid: TimeSpan,
    pub start_phase: u8,
    pub end_phase: u8,
    pub end_reason: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BirthAllocation {
    pub child_slot: u32,
    pub child_origin_cohort_id: String,
    pub child_sex_ordinal: u32,
    pub family_key: String,
    #[serde(deserialize_with = "required_nullable")]
    pub mother_slot: Option<u32>,
    #[serde(deserialize_with = "required_nullable")]
    pub father_slot: Option<u32>,
    #[serde(deserialize_with = "required_nullable")]
    pub parental_union_id: Option<String>,
    pub site_id: String,
    pub year: i32,
    pub phase: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeathAllocation {
    pub person_slot: u32,
    pub cause: String,
    pub site_id: String,
    pub year: i32,
    pub phase: u8,
}
/// One frozen household move. A `null` endpoint is a statistical port.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidenceTransfer {
    pub transfer_id: String,
    pub household_id: String,
    #[serde(deserialize_with = "required_nullable")]
    pub from_site: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub to_site: Option<String>,
    pub origin_slots: Vec<u32>,
    pub reason: String,
    pub source: SlotSource,
    #[serde(deserialize_with = "required_nullable")]
    pub operation_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub port_id: Option<String>,
    pub year: i32,
    pub phase: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardianAssignment {
    pub child_slot: u32,
    /// `null` under synthetic care: no invented guardian.
    #[serde(deserialize_with = "required_nullable")]
    pub guardian_slot: Option<u32>,
    pub source: String,
    pub year: i32,
    pub phase: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParentageRecord {
    pub child_slot: u32,
    #[serde(deserialize_with = "required_nullable")]
    pub mother_slot: Option<u32>,
    #[serde(deserialize_with = "required_nullable")]
    pub father_slot: Option<u32>,
    pub precision: SlotSource,
}
/// Site count observed at phase 80 of `year`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnualCensus {
    pub site_id: String,
    pub year: i32,
    pub population: u64,
    pub simulated: u64,
    pub projected: u64,
    pub births: u64,
    pub deaths: u64,
    pub incoming: u64,
    pub outgoing: u64,
    pub precision: String,
    pub coverage: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortResidenceCount {
    pub origin_cohort_id: String,
    pub site_id: String,
    pub sex: u8,
    pub source: SlotSource,
    pub year: i32,
    pub count: u64,
}
/// The part of the provider manifest the host compares with its plan. Everything
/// else stays as published.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderManifest {
    pub algorithm_id: String,
    pub projection_version: String,
    pub runtime_fingerprint: String,
    pub catalogue_fingerprints: CatalogueFingerprints,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}
/// Unverified provider file (§24.3). Only `validate_population` turns it into facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationResult {
    pub format: String,
    pub result_type: String,
    pub complete: bool,
    pub coverage: String,
    pub stage: String,
    pub plan_id: String,
    pub plan_fingerprint: String,
    pub result_fingerprint: String,
    pub timeline: TimeSpan,
    pub provider_manifest: ProviderManifest,
    pub candidate_table: CandidateTable,
    pub origin_cohorts: Vec<OriginCohort>,
    pub residence_index: Vec<SlotResidence>,
    pub residence_offsets: Vec<u32>,
    pub household_episodes: Vec<HouseholdEpisode>,
    pub union_allocations: Vec<UnionAllocation>,
    pub birth_allocations: Vec<BirthAllocation>,
    pub death_allocations: Vec<DeathAllocation>,
    pub residence_transfers: Vec<ResidenceTransfer>,
    pub guardian_assignments: Vec<GuardianAssignment>,
    pub parentage: Vec<ParentageRecord>,
    pub children_index: Vec<Vec<u32>>,
    pub annual_censuses: Vec<AnnualCensus>,
    pub cohort_residence_counts: Vec<CohortResidenceCount>,
    pub diagnostics: Value,
    /// The worker's own check: evidence, never a reason to skip the host's.
    pub validation_proof: Value,
}
/// Reads the compact provider file. Fingerprints stay with the host (§4.2).
pub fn parse_population_result(bytes: &[u8]) -> Result<PopulationResult, HistoryError> {
    super::population_wire::decode(bytes)
}
/// Reads the table form directly, as tests and tools write it.
pub fn parse_population_tables(bytes: &[u8]) -> Result<PopulationResult, HistoryError> {
    serde_json::from_slice(bytes).map_err(|e| {
        HistoryError::new(
            ErrorCode::InvalidPopulationResult,
            "population.shape_invalid",
        )
        .with_param("detail", e.to_string())
    })
}
/// Bytes behind `result_fingerprint`: the canonical object without that field.
pub fn result_canonical_bytes(mut result: Value) -> Result<Vec<u8>, HistoryError> {
    result
        .as_object_mut()
        .ok_or_else(|| {
            HistoryError::new(
                ErrorCode::InvalidPopulationResult,
                "population.shape_invalid",
            )
        })?
        .remove("result_fingerprint");
    canonical_bytes(&result)
}
/// Genealogical depth the plan guarantees (§8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct KinshipLimits {
    pub max_projection_ancestors: u64,
    pub max_projection_descendants: u64,
}
/// What the host itself checked, recorded next to the facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PopulationProof {
    pub validator: String,
    pub plan_fingerprint: String,
    pub result_fingerprint: String,
    pub candidate_slots: u64,
    pub site_years: u64,
}
/// A provider result that passed `validate_population`. Fields are private and
/// it has no deserializer: this function is the only way to obtain one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerifiedPopulation {
    result: PopulationResult,
    kinship_limits: KinshipLimits,
    proof: PopulationProof,
}
impl VerifiedPopulation {
    pub fn result(&self) -> &PopulationResult {
        &self.result
    }
    pub fn kinship_limits(&self) -> KinshipLimits {
        self.kinship_limits
    }
    pub fn proof(&self) -> &PopulationProof {
        &self.proof
    }
    /// Residence episodes of a slot, in chronological order.
    pub fn residences_of(&self, slot: u32) -> &[SlotResidence] {
        let slot = slot as usize;
        let offsets = &self.result.residence_offsets;
        &self.result.residence_index[offsets[slot] as usize..offsets[slot + 1] as usize]
    }
}

type Stamp = (i32, u8);
const BIRTH_PHASE: u8 = 20;
const DEATH_PHASE: u8 = 30;
const MOVE_PHASE: u8 = 40;
const CENSUS_PHASE: u8 = 80;

fn start_of(valid: TimeSpan, phase: u8) -> Stamp {
    (valid.start_year, phase)
}
fn end_of(valid: TimeSpan, phase: u8) -> Stamp {
    (valid.end_year, phase)
}
/// Flows counted from the registers, per site index and year.
type SiteFlows = BTreeMap<(usize, i32), u64>;

struct Audit<'a> {
    plan: &'a PopulationPlan,
    r: &'a PopulationResult,
    c: &'a CandidateTable,
    n: usize,
    /// Index of each planned site, in sorted order.
    sites: BTreeMap<&'a str, usize>,
    cohorts: BTreeMap<&'a str, &'a OriginCohort>,
    births: SiteFlows,
    deaths: SiteFlows,
    incoming: SiteFlows,
    outgoing: SiteFlows,
    /// Domain-level flows through ports, counted from the transfers alone.
    external_in: BTreeMap<i32, u64>,
    external_out: BTreeMap<i32, u64>,
}
impl<'a> Audit<'a> {
    fn fail(&self, constraint: &str, year: Option<i32>) -> HistoryError {
        HistoryError {
            owner: Some(self.plan.domain_id.clone()),
            at_year: year,
            plan_id: Some(self.plan.plan_id.clone()),
            ..HistoryError::new(ErrorCode::InvalidPopulationResult, constraint)
        }
    }
    fn require(&self, ok: bool, constraint: &str, year: Option<i32>) -> Result<(), HistoryError> {
        if ok {
            Ok(())
        } else {
            Err(self.fail(constraint, year))
        }
    }
    fn slot(&self, slot: u32) -> Result<usize, HistoryError> {
        let slot = slot as usize;
        self.require(slot < self.n, "population.dangling_reference", None)?;
        Ok(slot)
    }
    fn site(&self, id: &str, year: i32) -> Result<usize, HistoryError> {
        self.sites
            .get(id)
            .copied()
            .ok_or_else(|| self.fail("population.unknown_site", Some(year)))
    }
    fn residences(&self, slot: usize) -> &'a [SlotResidence] {
        let offsets = &self.r.residence_offsets;
        &self.r.residence_index[offsets[slot] as usize..offsets[slot + 1] as usize]
    }
    fn present_at(&self, slot: usize, stamp: Stamp, site: &str) -> bool {
        self.residences(slot).iter().any(|e| {
            start_of(e.valid, e.start_phase) <= stamp
                && stamp < end_of(e.valid, e.end_phase)
                && e.site_id == site
        })
    }

    fn header(&self) -> Result<(), HistoryError> {
        let (r, plan) = (self.r, self.plan);
        let complete = r.format == POPULATION_RESULT_FORMAT
            && r.result_type == "PopulationResult"
            && r.complete
            && r.coverage == "complete";
        self.require(complete, "population.not_complete", None)?;
        self.require(
            r.plan_id == plan.plan_id && r.plan_fingerprint == plan.plan_fingerprint,
            "population.foreign_plan",
            None,
        )?;
        self.require(
            r.timeline == plan.timeline,
            "population.foreign_timeline",
            None,
        )?;
        let manifest = &r.provider_manifest;
        if manifest.runtime_fingerprint != plan.provider.runtime_fingerprint {
            return Err(HistoryError {
                code: ErrorCode::ProviderVersionMismatch,
                ..self.fail("population.runtime_mismatch", None)
            });
        }
        self.require(
            manifest.algorithm_id == plan.provider.algorithm_id
                && manifest.projection_version == plan.provider.projection_version
                && manifest.catalogue_fingerprints == plan.catalogue_fingerprints,
            "population.provider_manifest_mismatch",
            None,
        )?;
        self.require(
            self.n as u64 <= plan.limits.max_candidate_slots
                && r.origin_cohorts.len() as u64 <= plan.limits.max_cohorts,
            "population.budget_exceeded",
            None,
        )
    }

    fn candidates(&self) -> Result<(), HistoryError> {
        let (c, timeline) = (self.c, self.plan.timeline);
        self.require(c.columns_aligned(), "population.columns_misaligned", None)?;
        self.require(
            c.origin_id.windows(2).all(|w| w[0] < w[1]),
            "population.identity_duplicated_or_unsorted",
            None,
        )?;
        self.require(
            self.cohorts.len() == self.r.origin_cohorts.len(),
            "population.cohort_invalid",
            None,
        )?;
        let ancestries: BTreeSet<_> = self
            .plan
            .resolved_profiles
            .ancestries
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        let mut ordinals: BTreeMap<(&str, u8), BTreeSet<u32>> = BTreeMap::new();
        for p in 0..self.n {
            let birth = c.birth_year[p];
            let origin = c.origin_cohort_id[p].as_str();
            let cohort = self
                .cohorts
                .get(origin)
                .ok_or_else(|| self.fail("population.cohort_invalid", Some(birth)))?;
            self.require(
                cohort.sealed
                    && cohort.birth_year == birth
                    && cohort.ancestry_id == c.ancestry_id[p]
                    && ancestries.contains(c.ancestry_id[p].as_str())
                    && c.sex[p] < 2,
                "population.cohort_invalid",
                Some(birth),
            )?;
            ordinals
                .entry((origin, c.sex[p]))
                .or_default()
                .insert(c.sex_ordinal[p]);
            let (from, to) = (c.coverage_start_year[p], c.coverage_end_year[p]);
            self.require(
                timeline.start_year <= from
                    && from <= to
                    && to <= timeline.end_year
                    && birth <= from,
                "population.coverage_invalid",
                Some(from),
            )?;
            let (death, departure) = (c.death_year[p], c.departure_year[p]);
            let state_ok = match c.departure_state[p] {
                DepartureState::Dead => {
                    departure.is_none() && death.is_some() && c.death_site_id[p].is_some()
                }
                // An exit closes coverage and never invents a death.
                DepartureState::DepartedExternal => {
                    death.is_none() && c.death_site_id[p].is_none() && departure == Some(to)
                }
                DepartureState::ResidentAtCoverageEnd => {
                    death.is_none()
                        && departure.is_none()
                        && c.death_site_id[p].is_none()
                        && to == timeline.end_year
                }
            };
            self.require(state_ok, "population.departure_state_invalid", Some(to))?;
            if let Some(year) = death {
                self.require(
                    birth <= year && year < timeline.end_year,
                    "population.chronology_invalid",
                    Some(year),
                )?;
            }
            for parent in [c.mother_slot[p], c.father_slot[p]].into_iter().flatten() {
                let parent = self.slot(parent)?;
                self.require(
                    c.birth_year[parent] < birth,
                    "population.chronology_invalid",
                    Some(birth),
                )?;
            }
        }
        for (id, cohort) in &self.cohorts {
            for sex in 0..2u8 {
                let expected = cohort.sex_counts[sex as usize];
                let complete = match ordinals.get(&(*id, sex)) {
                    None => expected == 0,
                    Some(set) => {
                        set.len() as u32 == expected
                            && set.last().is_none_or(|last| *last + 1 == expected)
                    }
                };
                self.require(
                    complete,
                    "population.cohort_invalid",
                    Some(cohort.birth_year),
                )?;
            }
        }
        Ok(())
    }

    /// Shape of the residence index only: `residences` relies on these offsets.
    fn residence_offsets(&self) -> Result<(), HistoryError> {
        let offsets = &self.r.residence_offsets;
        self.require(
            offsets.len() == self.n + 1
                && offsets.first() == Some(&0)
                && offsets.last().map(|o| *o as usize) == Some(self.r.residence_index.len())
                && offsets.windows(2).all(|w| w[0] < w[1]),
            "population.residence_offsets_invalid",
            None,
        )
    }

    /// One unbroken chain of residences per slot: never two at once, never none.
    fn residence_chains(&self) -> Result<(), HistoryError> {
        let (c, timeline) = (self.c, self.plan.timeline);
        for p in 0..self.n {
            let chain = self.residences(p);
            let mut previous: Option<Stamp> = None;
            for (i, row) in chain.iter().enumerate() {
                let a = start_of(row.valid, row.start_phase);
                let b = end_of(row.valid, row.end_phase);
                let year = Some(a.0);
                self.require(
                    row.person_slot as usize == p,
                    "population.residence_offsets_invalid",
                    year,
                )?;
                self.site(&row.site_id, a.0)?;
                self.require(
                    a <= b
                        && timeline.start_year <= a.0
                        && a.0 < timeline.end_year
                        && b.0 <= timeline.end_year,
                    "population.residence_interval_invalid",
                    year,
                )?;
                self.require(
                    previous.is_none_or(|end| end == a),
                    "population.residence_gap_or_overlap",
                    year,
                )?;
                self.require(
                    i + 1 == chain.len() || row.end_reason == ResidenceEnd::Transfer,
                    "population.residence_end_invalid",
                    year,
                )?;
                previous = Some(b);
            }
            let (first, last) = (&chain[0], &chain[chain.len() - 1]);
            self.require(
                first.valid.start_year == c.coverage_start_year[p],
                "population.coverage_invalid",
                Some(first.valid.start_year),
            )?;
            let end = end_of(last.valid, last.end_phase);
            let closed = match c.departure_state[p] {
                DepartureState::Dead => {
                    last.end_reason == ResidenceEnd::Death
                        && Some(end) == c.death_year[p].map(|y| (y, DEATH_PHASE))
                        && c.death_site_id[p].as_deref() == Some(&last.site_id)
                }
                DepartureState::DepartedExternal => {
                    last.end_reason == ResidenceEnd::DepartedExternal
                        && Some(end) == c.departure_year[p].map(|y| (y, MOVE_PHASE))
                }
                DepartureState::ResidentAtCoverageEnd => {
                    last.end_reason == ResidenceEnd::CoverageEnd && end == (timeline.end_year, 0)
                }
            };
            self.require(closed, "population.residence_end_invalid", Some(end.0))?;
        }
        Ok(())
    }

    fn households(&self) -> Result<(), HistoryError> {
        let mut spans: Vec<Vec<(Stamp, Stamp)>> = vec![Vec::new(); self.n];
        for row in &self.r.household_episodes {
            let a = start_of(row.valid, row.start_phase);
            let b = end_of(row.valid, row.end_phase);
            let year = Some(a.0);
            self.site(&row.site_id, a.0)?;
            let members: BTreeSet<_> = row.member_slots.iter().collect();
            self.require(
                a <= b && members.len() == row.member_slots.len(),
                "population.household_invalid",
                year,
            )?;
            for member in &row.member_slots {
                let member = self.slot(*member)?;
                if a < b {
                    spans[member].push((a, b));
                }
            }
        }
        for person in &mut spans {
            person.sort_unstable();
            self.require(
                person.windows(2).all(|w| w[0].1 <= w[1].0),
                "population.household_overlap",
                person.first().map(|span| span.0 .0),
            )?;
        }
        Ok(())
    }

    fn unions(&self) -> Result<BTreeMap<&'a str, &'a UnionAllocation>, HistoryError> {
        let (c, end) = (self.c, self.plan.timeline.end_year);
        let mut unions = BTreeMap::new();
        for row in &self.r.union_allocations {
            let a = start_of(row.valid, row.start_phase);
            let b = end_of(row.valid, row.end_phase);
            let year = Some(a.0);
            self.site(&row.site_at_formation, a.0)?;
            let two = row.partner_slots.len() == 2 && row.partner_slots[0] != row.partner_slots[1];
            self.require(
                two && a <= b && unions.insert(row.union_id.as_str(), row).is_none(),
                "population.union_invalid",
                year,
            )?;
            for partner in &row.partner_slots {
                let p = self.slot(*partner)?;
                let limit = match (c.death_year[p], c.departure_year[p]) {
                    (Some(death), _) => (death, DEATH_PHASE),
                    (None, Some(departure)) => (departure, MOVE_PHASE),
                    (None, None) => (end, 0),
                };
                self.require(
                    b <= limit && c.birth_year[p] <= a.0,
                    "population.union_outside_life",
                    year,
                )?;
                if row.source != UnionSource::Simulated {
                    self.require(
                        self.present_at(p, a, &row.site_at_formation),
                        "population.union_not_coresident",
                        year,
                    )?;
                }
            }
        }
        Ok(unions)
    }

    fn births(&mut self, unions: &BTreeMap<&str, &UnionAllocation>) -> Result<(), HistoryError> {
        let (c, r) = (self.c, self.r);
        let mut registered = vec![false; self.n];
        for row in &r.birth_allocations {
            let year = Some(row.year);
            let p = self.slot(row.child_slot)?;
            let site = self.site(&row.site_id, row.year)?;
            self.require(
                !registered[p]
                    && row.year == c.birth_year[p]
                    && row.phase == BIRTH_PHASE
                    && row.family_key == c.family_key[p]
                    && row.child_origin_cohort_id == c.origin_cohort_id[p]
                    && row.child_sex_ordinal == c.sex_ordinal[p]
                    && row.mother_slot == c.mother_slot[p]
                    && row.father_slot == c.father_slot[p]
                    && self.residences(p)[0].site_id == row.site_id,
                "population.birth_invalid",
                year,
            )?;
            let union = row
                .parental_union_id
                .as_deref()
                .and_then(|id| unions.get(id))
                .ok_or_else(|| self.fail("population.birth_without_union", year))?;
            let partners: BTreeSet<_> = union.partner_slots.iter().map(|s| Some(*s)).collect();
            self.require(
                union.family_key == row.family_key
                    && union.valid.start_year < row.year
                    && end_of(union.valid, union.end_phase) > (row.year, BIRTH_PHASE)
                    && partners == BTreeSet::from([row.mother_slot, row.father_slot]),
                "population.birth_without_union",
                year,
            )?;
            for parent in [row.mother_slot, row.father_slot].into_iter().flatten() {
                let parent = self.slot(parent)?;
                self.require(
                    self.present_at(parent, (row.year, BIRTH_PHASE), &row.site_id),
                    "population.birth_parents_absent",
                    year,
                )?;
            }
            registered[p] = true;
            *self.births.entry((site, row.year)).or_default() += 1;
        }
        for (p, registered) in registered.iter().enumerate() {
            let cohort = self.cohorts[c.origin_cohort_id[p].as_str()];
            self.require(
                (cohort.origin_type == OriginType::Birth) == *registered,
                "population.birth_invalid",
                Some(c.birth_year[p]),
            )?;
        }
        Ok(())
    }

    fn deaths(&mut self) -> Result<(), HistoryError> {
        let (c, r) = (self.c, self.r);
        let mut registered = vec![false; self.n];
        for row in &r.death_allocations {
            let p = self.slot(row.person_slot)?;
            let site = self.site(&row.site_id, row.year)?;
            self.require(
                !registered[p]
                    && row.phase == DEATH_PHASE
                    && c.death_year[p] == Some(row.year)
                    && c.death_site_id[p].as_deref() == Some(&row.site_id),
                "population.death_invalid",
                Some(row.year),
            )?;
            registered[p] = true;
            *self.deaths.entry((site, row.year)).or_default() += 1;
        }
        for (p, registered) in registered.iter().enumerate() {
            self.require(
                c.death_year[p].is_some() == *registered,
                "population.death_invalid",
                c.death_year[p],
            )?;
        }
        Ok(())
    }

    fn transfers(&mut self) -> Result<(), HistoryError> {
        let (c, r, plan) = (self.c, self.r, self.plan);
        let ports: BTreeSet<_> = plan
            .boundary_ports
            .iter()
            .map(|p| p.port_id.as_str())
            .collect();
        let operations: BTreeSet<_> = plan
            .historical_program
            .iter()
            .map(|op| op.application_key().2)
            .collect();
        let mut episodes: BTreeMap<&str, Vec<&HouseholdEpisode>> = BTreeMap::new();
        for episode in &r.household_episodes {
            episodes
                .entry(&episode.household_id)
                .or_default()
                .push(episode);
        }
        let mut ids = BTreeSet::new();
        let mut moved = BTreeSet::new();
        for row in &r.residence_transfers {
            let (year, stamp) = (Some(row.year), (row.year, MOVE_PHASE));
            let members: BTreeSet<_> = row.origin_slots.iter().copied().collect();
            self.require(
                ids.insert(row.transfer_id.as_str())
                    && row.phase == MOVE_PHASE
                    && row.from_site != row.to_site
                    && !members.is_empty()
                    && members.len() == row.origin_slots.len(),
                "population.transfer_invalid",
                year,
            )?;
            let external = row.from_site.is_none() || row.to_site.is_none();
            self.require(
                row.port_id.as_deref().is_none_or(|p| ports.contains(p))
                    && row
                        .operation_id
                        .as_deref()
                        .is_none_or(|op| operations.contains(op))
                    && (!external || row.port_id.is_some() || row.operation_id.is_some()),
                "population.transfer_unknown_source",
                year,
            )?;
            let frozen = episodes.get(row.household_id.as_str()).is_some_and(|rows| {
                rows.iter().any(|h| {
                    let endpoint = if row.from_site.is_some() {
                        end_of(h.valid, h.end_phase)
                    } else {
                        start_of(h.valid, h.start_phase)
                    };
                    endpoint == stamp
                        && h.member_slots.iter().copied().collect::<BTreeSet<_>>() == members
                })
            });
            self.require(frozen, "population.transfer_membership_differs", year)?;
            for member in &row.origin_slots {
                let p = self.slot(*member)?;
                self.require(
                    moved.insert((p, row.year)),
                    "population.transfer_moved_twice",
                    year,
                )?;
                let chain = self.residences(p);
                if let Some(from) = &row.from_site {
                    let site = self.site(from, row.year)?;
                    self.require(
                        chain
                            .iter()
                            .any(|e| end_of(e.valid, e.end_phase) == stamp && &e.site_id == from),
                        "population.transfer_without_residence",
                        year,
                    )?;
                    *self.outgoing.entry((site, row.year)).or_default() += 1;
                } else {
                    *self.external_in.entry(row.year).or_default() += 1;
                }
                if let Some(to) = &row.to_site {
                    let site = self.site(to, row.year)?;
                    self.require(
                        chain
                            .iter()
                            .any(|e| start_of(e.valid, e.start_phase) == stamp && &e.site_id == to),
                        "population.transfer_without_residence",
                        year,
                    )?;
                    *self.incoming.entry((site, row.year)).or_default() += 1;
                } else {
                    self.require(
                        c.departure_year[p] == Some(row.year),
                        "population.exit_without_departure",
                        year,
                    )?;
                    *self.external_out.entry(row.year).or_default() += 1;
                }
            }
        }
        Ok(())
    }

    /// Family links rebuilt from the candidate columns must match both registers.
    fn kinship(&self) -> Result<(), HistoryError> {
        let c = self.c;
        let mut children = vec![Vec::new(); self.n];
        let mut parentage = Vec::new();
        for p in 0..self.n {
            let (mother, father) = (c.mother_slot[p], c.father_slot[p]);
            for parent in [mother, father].into_iter().flatten() {
                children[self.slot(parent)?].push(p as u32);
            }
            if mother.is_some() || father.is_some() {
                parentage.push(ParentageRecord {
                    child_slot: p as u32,
                    mother_slot: mother,
                    father_slot: father,
                    precision: c.source[p],
                });
            }
        }
        self.require(
            children == self.r.children_index && parentage == self.r.parentage,
            "population.kinship_not_reciprocal",
            None,
        )?;
        for row in &self.r.guardian_assignments {
            let child = self.slot(row.child_slot)?;
            let guardian = row.guardian_slot.map(|g| self.slot(g)).transpose()?;
            self.require(
                guardian != Some(child),
                "population.guardian_invalid",
                Some(row.year),
            )?;
        }
        Ok(())
    }

    /// Opening population per site index: the founders, before the first census.
    fn founders(&self) -> Result<Vec<i64>, HistoryError> {
        let c = self.c;
        let mut founders = vec![0i64; self.sites.len()];
        for p in 0..self.n {
            if self.cohorts[c.origin_cohort_id[p].as_str()].origin_type == OriginType::Founder {
                founders[self.sites[self.residences(p)[0].site_id.as_str()]] += 1;
            }
        }
        for settlement in &self.plan.settlements {
            self.require(
                founders[self.sites[settlement.site_id.as_str()]] as u64
                    == settlement.initial_population,
                "population.founders_differ_from_plan",
                Some(self.plan.timeline.start_year),
            )?;
        }
        Ok(founders)
    }

    /// Episode boundaries, closings first so a person leaves one, then enters the next.
    fn episode_events(&self) -> Vec<EpisodeEvent> {
        let mut events = Vec::new();
        let mut add = |a: Stamp, b: Stamp, register, person, site: &str| {
            if a < b {
                let site = self.sites[site];
                events.push((a, 1, register, person, site));
                events.push((b, -1, register, person, site));
            }
        };
        for row in &self.r.residence_index {
            let a = start_of(row.valid, row.start_phase);
            let b = end_of(row.valid, row.end_phase);
            add(a, b, Register::Residence, row.person_slot, &row.site_id);
        }
        for row in &self.r.household_episodes {
            let a = start_of(row.valid, row.start_phase);
            let b = end_of(row.valid, row.end_phase);
            for member in &row.member_slots {
                add(a, b, Register::Household, *member, &row.site_id);
            }
        }
        events.sort_unstable();
        events
    }

    fn census_grid(&self) -> Result<BTreeMap<(&'a str, i32), &'a AnnualCensus>, HistoryError> {
        let timeline = self.plan.timeline;
        let census: BTreeMap<_, _> = self
            .r
            .annual_censuses
            .iter()
            .map(|row| ((row.site_id.as_str(), row.year), row))
            .collect();
        let years = (timeline.end_year - timeline.start_year) as usize;
        self.require(
            census.len() == self.r.annual_censuses.len()
                && census.len() == self.sites.len() * years
                && census
                    .keys()
                    .all(|(site, year)| self.sites.contains_key(site) && timeline.contains(*year)),
            "population.census_grid_incomplete",
            None,
        )?;
        Ok(census)
    }

    /// Sparse origin/site/sex counts of one year against the resident slots.
    fn sparse_counts(
        &self,
        year: i32,
        residence: &[Option<usize>],
        rows: Vec<&CohortResidenceCount>,
    ) -> Result<(), HistoryError> {
        let c = self.c;
        let mut expected: BTreeMap<(&str, usize, u8, SlotSource), u64> = BTreeMap::new();
        for (p, site) in residence.iter().enumerate() {
            if let Some(site) = *site {
                *expected
                    .entry((&c.origin_cohort_id[p], site, c.sex[p], c.source[p]))
                    .or_default() += 1;
            }
        }
        for row in rows {
            let site = self.site(&row.site_id, year)?;
            let key = (row.origin_cohort_id.as_str(), site, row.sex, row.source);
            self.require(
                expected.remove(&key) == Some(row.count),
                "population.sparse_counts_differ",
                Some(year),
            )?;
        }
        self.require(
            expected.is_empty(),
            "population.sparse_counts_differ",
            Some(year),
        )
    }

    /// Walks residences and households to each phase-80 census, then compares
    /// every published count with what the tables alone imply (§24.3).
    fn balances(&self) -> Result<u64, HistoryError> {
        let (r, c, timeline) = (self.r, self.c, self.plan.timeline);
        let events = self.episode_events();
        let census = self.census_grid()?;
        let mut sparse: BTreeMap<i32, Vec<&CohortResidenceCount>> = BTreeMap::new();
        for row in &r.cohort_residence_counts {
            sparse.entry(row.year).or_default().push(row);
        }
        let site_ids: Vec<&str> = self.sites.keys().copied().collect();
        let mut previous = self.founders()?;
        let mut domain: i64 = previous.iter().sum();
        let mut residence: Vec<Option<usize>> = vec![None; self.n];
        let mut household: Vec<Option<usize>> = vec![None; self.n];
        let mut cursor = 0;
        for year in timeline.start_year..timeline.end_year {
            let at = Some(year);
            while cursor < events.len() && events[cursor].0 <= (year, CENSUS_PHASE) {
                let (stamp, delta, register, person, site) = events[cursor];
                let (state, key) = match register {
                    Register::Residence => (
                        &mut residence[person as usize],
                        "population.residence_gap_or_overlap",
                    ),
                    Register::Household => (
                        &mut household[person as usize],
                        "population.household_overlap",
                    ),
                };
                let consistent = if delta > 0 {
                    state.replace(site).is_none()
                } else {
                    state.take() == Some(site)
                };
                self.require(consistent, key, Some(stamp.0))?;
                cursor += 1;
            }
            self.require(
                residence == household,
                "population.resident_without_household",
                at,
            )?;
            self.sparse_counts(year, &residence, sparse.remove(&year).unwrap_or_default())?;
            let mut counts = vec![[0u64; 2]; site_ids.len()];
            for (p, site) in residence.iter().enumerate() {
                if let Some(site) = *site {
                    counts[site][(c.source[p] == SlotSource::Projected) as usize] += 1;
                }
            }
            let mut total = 0i64;
            let mut natural = 0i64;
            for (i, site) in site_ids.iter().enumerate() {
                let row = census[&(*site, year)];
                let flow = |table: &SiteFlows| table.get(&(i, year)).copied().unwrap_or(0);
                let (births, deaths) = (flow(&self.births), flow(&self.deaths));
                let (incoming, outgoing) = (flow(&self.incoming), flow(&self.outgoing));
                let [simulated, projected] = counts[i];
                let population = simulated + projected;
                self.require(
                    population as i64
                        == previous[i] + births as i64 - deaths as i64 + incoming as i64
                            - outgoing as i64,
                    "population.balance_not_closed",
                    at,
                )?;
                self.require(
                    row.population == population
                        && row.simulated == simulated
                        && row.projected == projected
                        && row.births == births
                        && row.deaths == deaths
                        && row.incoming == incoming
                        && row.outgoing == outgoing,
                    "population.census_differs",
                    at,
                )?;
                previous[i] = population as i64;
                total += population as i64;
                natural += births as i64 - deaths as i64;
            }
            // Local moves cancel at domain level: only ports change the total.
            let port = |table: &BTreeMap<i32, u64>| table.get(&year).copied().unwrap_or(0) as i64;
            domain += natural + port(&self.external_in) - port(&self.external_out);
            self.require(total == domain, "population.domain_balance_not_closed", at)?;
        }
        self.require(sparse.is_empty(), "population.sparse_counts_differ", None)?;
        Ok(census.len() as u64)
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Register {
    Residence,
    Household,
}
type EpisodeEvent = (Stamp, i8, Register, u32, usize);

/// Checks a provider result against its plan from the tables alone (§14.1, §24.3):
/// references, coverage, chronology, single residence, reciprocal kinship and
/// balances per site and per domain. A published proof changes nothing here. The
/// host has already checked the file and result fingerprints (§4.2).
pub fn validate_population(
    plan: &PopulationPlan,
    mut result: PopulationResult,
) -> Result<VerifiedPopulation, HistoryError> {
    let mut sites: Vec<_> = plan
        .settlements
        .iter()
        .map(|s| s.site_id.as_str())
        .collect();
    sites.sort_unstable();
    let mut audit = Audit {
        plan,
        r: &result,
        c: &result.candidate_table,
        n: result.candidate_table.len(),
        sites: sites.iter().enumerate().map(|(i, s)| (*s, i)).collect(),
        cohorts: result
            .origin_cohorts
            .iter()
            .map(|c| (c.origin_cohort_id.as_str(), c))
            .collect(),
        births: BTreeMap::new(),
        deaths: BTreeMap::new(),
        incoming: BTreeMap::new(),
        outgoing: BTreeMap::new(),
        external_in: BTreeMap::new(),
        external_out: BTreeMap::new(),
    };
    audit.header()?;
    audit.candidates()?;
    audit.residence_offsets()?;
    audit.residence_chains()?;
    audit.households()?;
    let unions = audit.unions()?;
    audit.births(&unions)?;
    audit.deaths()?;
    audit.transfers()?;
    audit.kinship()?;
    let site_years = audit.balances()?;
    let proof = PopulationProof {
        validator: POPULATION_VALIDATOR.into(),
        plan_fingerprint: result.plan_fingerprint.clone(),
        result_fingerprint: result.result_fingerprint.clone(),
        candidate_slots: result.candidate_table.len() as u64,
        site_years,
    };
    // Sparse counts exist to be audited. Nothing reads them afterwards and they are the
    // largest table; the published file keeps them.
    result.cohort_residence_counts = Vec::new();
    Ok(VerifiedPopulation {
        result,
        kinship_limits: KinshipLimits {
            max_projection_ancestors: plan.limits.max_projection_ancestors,
            max_projection_descendants: plan.limits.max_projection_descendants,
        },
        proof,
    })
}
