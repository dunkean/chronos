//! Plan autonome de population : forme du schéma 1 et contrôles S0.
use super::atlas::{
    covers, unique_ids, valid_weights, validate_mobility_edge, MobilityEdge, OccupationEpisode,
    OccupationState,
};
use super::model::{ErrorCode, HistoryError, TimeSpan};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

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
fn required_nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}
fn present_optional<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
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

/// Références du canon et des packs déjà vérifiés par l'hôte. Aucun accès
/// implicite à un fichier, worker ou dépôt voisin pendant la validation.
#[derive(Clone, Debug)]
pub struct PlanReferences {
    pub catalogue_fingerprints: CatalogueFingerprints,
    pub event_years: BTreeMap<String, i32>,
    pub office_ids: BTreeSet<String>,
    /// history_family/variant -> demography_society_id résolu par S1.
    pub history_cultures: BTreeMap<String, String>,
    pub culture_ids: BTreeSet<String>,
    pub activity_ids: BTreeSet<String>,
    pub contacts: BTreeMap<String, super::atlas::ExternalContact>,
    /// Bornes de l'atlas figé sur tout l'horizon, pas la seule population
    /// observée dans la requête ou aux marqueurs.
    pub site_population_expected_max: BTreeMap<String, u64>,
}

/// Valide un contrat de données ordinaire S0–S5, sans annoncer que son
/// fournisseur est exécuté par S0. Les extensions S6–S8 sont refusées.
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
fn validate_plan_inner(p: &PopulationPlan, refs: &PlanReferences) -> Result<(), HistoryError> {
    // Le schéma normatif est embarqué à la compilation : aucune E/S runtime.
    // Les contrôles de forme restent ainsi identiques au contrat JSON publié.
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../docs/design/chronos-social-v2/population-plan.schema.json"
    ))
    .expect("schéma livré valide");
    let value = serde_json::to_value(p).map_err(|e| invalid(e.to_string()))?;
    check_shape(&value, &schema, &schema, "plan")?;
    p.timeline.validate(false)?;
    let world_seed = decimal_seed(&p.world_seed)?;
    if decimal_seed(&p.seed)?
        != super::ids::random_semantics_v2::domain_seed(
            world_seed,
            &p.domain_id,
            &p.provider.algorithm_id,
        )
    {
        return Err(invalid("graine de domaine incohérente"));
    }
    if p.catalogue_fingerprints != refs.catalogue_fingerprints {
        return Err(invalid(
            "empreintes des catalogues différentes des références",
        ));
    }
    if p.provider.mode != ProviderMode::LineageHybrid {
        return Err(unsupported("fournisseur procédural S8"));
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
        return Err(unsupported("capacité non livrée ou inconnue"));
    }
    if !capabilities.contains("historical_soft_v1")
        || (!p.role_demands.is_empty() && !capabilities.contains("candidate_slots_v1"))
        || (!p.boundary_ports.is_empty() && !capabilities.contains("statistical_reservoir_v1"))
    {
        return Err(unsupported("capacité obligatoire absente"));
    }
    if !p.fixed_people.is_empty() || !p.exact_constraints.is_empty() {
        return Err(unsupported("personnes fixées et contraintes exactes S6"));
    }
    if p.core_policy != CorePolicy::FreeTry3ThenProjected {
        return Err(unsupported("noyau requis S6"));
    }
    let limits = &p.limits;
    if limits.core_target_share_bps > limits.core_max_share_bps
        || limits.max_core_living > limits.max_core_historical
    {
        return Err(invalid("plafonds de noyau incohérents"));
    }
    if limits.max_candidate_slots > 250_000
        || limits.max_halo_contacts > 16
        || limits.max_result_bytes > 536_870_912
        || limits.max_attempts != 4
        || p.boundary_ports.len() as u64 > limits.max_halo_contacts
    {
        return Err(HistoryError::new(
            ErrorCode::BudgetExceeded,
            "budget du prototype ou politique de quatre essais dépassé",
        ));
    }
    let profiles = &p.resolved_profiles;
    if profiles.society.id != p.demography_society_id {
        return Err(invalid("profil social du domaine incohérent"));
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
            return Err(invalid("ID de profil dupliqué"));
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
        return Err(invalid("rangs sociaux non contigus ou dupliqués"));
    }
    check_weight_refs(&profiles.society.status_weights, &status_ids)?;
    check_weight_refs(&profiles.society.core_status_weights, &status_ids)?;
    check_parameters(&profiles.society.parameters, true)?;
    for ancestry in &profiles.ancestries {
        if ancestry.reproduction_mode != ReproductionMode::Biparental {
            return Err(unsupported("reproduction personnalisée non livrée"));
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
            return Err(invalid(
                "profil extérieur : âges ou tailles de ménage invalides",
            ));
        }
    }
    if !unique_ids(p.settlements.iter().map(|s| s.site_id.as_str()))
        || !unique_ids(p.boundary_ports.iter().map(|s| s.port_id.as_str()))
        || !unique_ids(p.role_demands.iter().map(|s| s.slot_id.as_str()))
        || !unique_ids(p.mobility_edges.iter().map(|s| s.edge_id.as_str()))
    {
        return Err(invalid(
            "identités de sites, ports, slots ou routes dupliquées",
        ));
    }
    let local_ids: BTreeSet<_> = p.settlements.iter().map(|s| s.provider_local_id).collect();
    if local_ids.len() != p.settlements.len() {
        return Err(invalid("identifiant local fournisseur dupliqué"));
    }
    let sites: BTreeSet<_> = p.settlements.iter().map(|s| s.site_id.as_str()).collect();
    let frames: BTreeSet<_> = p.settlements.iter().map(|s| &s.coordinate_frame).collect();
    if frames.len() != 1 {
        return Err(invalid("plusieurs référentiels dans un domaine"));
    }
    let mut max_population: BTreeMap<_, _> = p
        .settlements
        .iter()
        .map(|s| (s.site_id.as_str(), s.initial_population))
        .collect();
    let cultures: BTreeSet<_> = refs.culture_ids.iter().map(String::as_str).collect();
    let activities: BTreeSet<_> = refs.activity_ids.iter().map(String::as_str).collect();
    for s in &p.settlements {
        check_weight_refs(&s.ancestry_weights, &ancestry_ids)?;
        if s.foundation_year > p.timeline.start_year
            || s.occupation_episodes
                .iter()
                .any(|e| e.state == OccupationState::Abandoned)
        {
            return Err(unsupported("sites dynamiques S8"));
        }
        if !covers(s.occupation_episodes.iter().map(|e| e.valid), p.timeline) {
            return Err(invalid(
                "épisodes d'occupation incomplets, non ordonnés ou chevauchants",
            ));
        }
        if !profiles
            .kinship
            .iter()
            .any(|k| k.id == s.kinship_profile_id)
            || !refs
                .history_cultures
                .get(&s.history_culture_id)
                .is_some_and(|society| society == &p.demography_society_id)
            || s.history_culture_id.split('/').count() != 2
            || s.history_culture_id.split('/').any(str::is_empty)
        {
            return Err(invalid("raccord culturel ou profil de parenté absent"));
        }
        for e in &s.occupation_episodes {
            if !valid_weights(&e.culture_weights)
                || !valid_weights(&e.activity_weights)
                || e.culture_weights
                    .keys()
                    .any(|k| !cultures.contains(k.as_str()))
                || e.activity_weights
                    .keys()
                    .any(|k| !activities.contains(k.as_str()))
            {
                return Err(invalid("poids ou références d'occupation invalides"));
            }
        }
    }
    let mut marker_dates = BTreeSet::new();
    for m in &p.baseline_markers {
        if !sites.contains(m.site_id.as_str())
            || !p.timeline.contains(m.year)
            || !marker_dates.insert((&m.site_id, m.year))
        {
            return Err(invalid("marqueur absent, hors période ou dupliqué"));
        }
        let max = max_population
            .get_mut(m.site_id.as_str())
            .expect("site vérifié");
        *max = (*max).max(m.population);
    }
    let mut domain_max = 0u128;
    for (site, observed_max) in max_population {
        let bound = refs
            .site_population_expected_max
            .get(site)
            .ok_or_else(|| invalid("borne d'atlas du site absente"))?;
        if observed_max > *bound {
            return Err(invalid("population du plan au-delà de la borne d'atlas"));
        }
        domain_max += u128::from(*bound);
    }
    if domain_max > 20_000 {
        return Err(HistoryError::new(
            ErrorCode::BudgetExceeded,
            "plus de 20 000 habitants attendus maximaux",
        ));
    }
    let event = |id: &str, year: i32| -> Result<(), HistoryError> {
        if refs.event_years.get(id).is_none_or(|y| *y > year) {
            Err(invalid("événement source absent ou futur"))
        } else {
            Ok(())
        }
    };
    let span = |s: TimeSpan| -> Result<(), HistoryError> {
        s.validate(false)?;
        if !p.timeline.encloses(s) {
            Err(invalid("intervalle hors du plan"))
        } else {
            Ok(())
        }
    };
    let mut shocks = BTreeSet::new();
    for s in &p.shocks {
        span(s.valid)?;
        event(&s.source_event_id, s.valid.start_year)?;
        for id in &s.site_ids {
            if !sites.contains(id.as_str())
                || !shocks.insert((&s.source_event_id, id, s.valid.start_year, s.valid.end_year))
            {
                return Err(invalid("choc local dupliqué ou site absent"));
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
                    return Err(unsupported("mouvement exact S6"));
                }
                event(source_event_id, *year)?;
                if !ops.insert(op_id)
                    || !p.timeline.contains(*year)
                    || !sites.contains(origin_site_id.as_str())
                    || destination_site_id.is_some() == destination_port_id.is_some()
                {
                    return Err(invalid("mouvement, date ou destination invalide"));
                }
                if let Some(dest) = destination_site_id {
                    if dest == origin_site_id || !sites.contains(dest.as_str()) {
                        return Err(invalid("destination de mouvement absente ou identique"));
                    }
                }
                if let Some(port) = destination_port_id {
                    if !p.boundary_ports.iter().any(|p| {
                        &p.port_id == port
                            && p.valid.contains(*year)
                            && &p.entry_site_id == origin_site_id
                    }) {
                        return Err(invalid("port de destination absent ou incompatible"));
                    }
                }
            }
            _ => return Err(unsupported("opération personnelle S6")),
        }
    }
    for r in &p.role_demands {
        if r.policy != RolePolicy::BestEffort {
            return Err(unsupported("rôle requis S6"));
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
            return Err(invalid("demande de rôle invalide"));
        }
    }
    for port in &p.boundary_ports {
        if port.mode != PortMode::StatisticalReservoir || port.bridge_plan_id.is_some() {
            return Err(unsupported("pont nominatif S7"));
        }
        span(port.valid)?;
        let contact = refs
            .contacts
            .get(&port.contact_id)
            .ok_or_else(|| invalid("contact absent"))?;
        if !sites.contains(port.entry_site_id.as_str())
            || contact.entry_site_id != port.entry_site_id
            || contact.external_context_id != port.external_context_id
            || contact.external_profile_id != port.external_profile_id
            || contact.population_expected != port.population_expected
            || !contact.valid.encloses(port.valid)
            || !profiles
                .external
                .iter()
                .any(|e| e.id == port.external_profile_id)
            || !covers(port.rate_episodes.iter().map(|e| e.valid), port.valid)
        {
            return Err(invalid("port, contact ou épisodes de taux incohérents"));
        }
    }
    for edge in &p.mobility_edges {
        validate_mobility_edge(edge, p.timeline, &sites)?;
    }
    Ok(())
}
fn decimal_seed(s: &str) -> Result<u64, HistoryError> {
    let n: u64 = s.parse().map_err(|_| invalid("graine hors u64"))?;
    if n.to_string() != s {
        return Err(invalid("graine décimale non normalisée"));
    }
    Ok(n)
}
fn positive_weights(weights: &BTreeMap<String, u64>) -> Result<(), HistoryError> {
    if weights.is_empty() || !weights.values().any(|v| *v > 0) {
        Err(invalid("poids de somme nulle"))
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
        Err(invalid("référence de poids absente"))
    } else {
        Ok(())
    }
}

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
        return Err(invalid("paramètre de profil inconnu ou manquant"));
    }
    for (key, value) in parameters {
        if key == "residence" {
            // Seule valeur figée par l'exemple ; les autres modes attendent
            // le dictionnaire exécutable S1, sans en inventer ici la liste.
            if !matches!(value, ParameterValue::Text(s) if s == "either") {
                return Err(unsupported(
                    "mode de résidence non spécifié par le contrat initial",
                ));
            }
            continue;
        }
        let n = value
            .integer()
            .ok_or_else(|| invalid("paramètre numérique mal typé"))?;
        if n < 0
            || (key.ends_with("_bps") && n > 10_000)
            || (key.ends_with("_ppm") && n > 10_000_000)
            || (key.ends_with("_mm") && n > 9_000_000_000_000_000)
        {
            return Err(invalid("unité ou borne de paramètre invalide"));
        }
    }
    let n = |key: &str| parameters[key].integer().expect("type contrôlé");
    if society {
        if n("marriage_min_age") > n("marriage_max_age")
            || n("distance_scale_mm") == 0
            || n("matching_attempts") == 0
            || n("max_neighbors") == 0
        {
            return Err(invalid("âges, échelle ou nombre d'essais incohérents"));
        }
    } else if n("fertility_min_age") > n("fertility_peak_age")
        || n("fertility_peak_age") > n("fertility_max_age")
        || n("fertility_max_age") > n("max_age")
        || n("child_max_age") > n("max_age")
        || n("fertility_width_years") == 0
        || n("birth_spacing_years") == 0
    {
        return Err(invalid("âges ou périodes biologiques incohérents"));
    }
    Ok(())
}

// Interpréteur privé limité aux mots-clés présents dans le schéma livré.
// Les types Rust assurent déjà la forme ; ce passage contrôle les bornes,
// cardinalités et constantes, y compris après construction directe en Rust.
fn check_shape(v: &Value, s: &Value, root: &Value, path: &str) -> Result<(), HistoryError> {
    let fail = || invalid(format!("forme ou borne du schéma : {path}"));
    if let Some(r) = s.get("$ref").and_then(Value::as_str) {
        return check_shape(
            v,
            root.pointer(&r[1..]).expect("référence locale livrée"),
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
                .any(|k| !obj.contains_key(k.as_str().expect("nom de champ")))
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
