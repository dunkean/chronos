//! Faits, mesures et diagnostics v2 (§3–5, §7.3, §13).
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! string_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
pub(crate) use string_enum;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeSpan {
    pub start_year: i32,
    pub end_year: i32,
}
impl TimeSpan {
    pub fn contains(self, year: i32) -> bool {
        self.start_year <= year && year < self.end_year
    }
    pub fn encloses(self, other: Self) -> bool {
        self.start_year <= other.start_year && other.end_year <= self.end_year
    }
    /// Un intervalle de vie vide est permis ; les autres intervalles sont stricts.
    pub fn validate(self, allow_empty: bool) -> Result<(), HistoryError> {
        if self.start_year < -100_000
            || self.end_year > 100_000
            || self.start_year > self.end_year
            || (!allow_empty && self.start_year == self.end_year)
        {
            Err(HistoryError::new(
                ErrorCode::InvalidInterval,
                "intervalle semi-ouvert invalide",
            ))
        } else {
            Ok(())
        }
    }
}
string_enum!(ProvenanceStatus {
    Imported,
    Derived,
    Synthetic,
    Unavailable
});
string_enum!(PrecisionMode {
    Expected,
    Realized,
    Exact
});
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Precision {
    pub mode: PrecisionMode,
}
string_enum!(Coverage {
    Aggregate,
    Partial,
    Complete
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub status: ProvenanceStatus,
    pub source_ref: String,
    pub rule_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountMeasure {
    pub value: Option<u64>,
    pub unit: String,
    pub precision: Precision,
    pub coverage: Coverage,
    pub basis: String,
    pub at_year: i32,
}
string_enum!(EntityNature {
    Person,
    OrganizedGroup,
    Community,
    Category,
    Population
});
string_enum!(Agency {
    None,
    Individual,
    Delegated,
    Governed
});
string_enum!(Lifecycle {
    Active,
    Dormant,
    Dissolved
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialEntity {
    pub id: String,
    pub semantic_address: super::ids::SemanticAddress,
    pub kind: String,
    pub archetype_id: String,
    pub nature: EntityNature,
    pub agency: Agency,
    pub owner_scope: String,
    pub existence: TimeSpan,
    pub culture_refs: Vec<String>,
    pub institution_ref: Option<String>,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityState {
    pub entity_id: String,
    pub at_year: i32,
    pub lifecycle: Lifecycle,
    pub member_count: CountMeasure,
    pub cohesion_bps: u16,
    pub legitimacy_bps: u16,
    pub wealth_index: i64,
    pub capabilities: Vec<String>,
    pub goals: Vec<String>,
    pub assets: Vec<String>,
    pub footprint_refs: Vec<String>,
    pub institution_version: String,
}
string_enum!(Predicate {
    ComponentOf,
    MemberOf,
    AffiliatedWith,
    Represents,
    ParentOf,
    UnionWith,
    AdoptedBy,
    Trusts,
    Helps,
    Rivals,
    SharesPastWith,
    EmployedBy,
    ApprenticeOf,
    PartnersWith,
    Supplies,
    HoldsOffice,
    Governs,
    Commands,
    Protects,
    OwesAllegianceTo,
    Owns,
    Controls,
    Claims,
    Owes,
    TradesWith,
    AlliedWith,
    InConflictWith,
    SuccessorOf,
    SplitFrom,
    MergedInto
});
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ObjectRef {
    Entity(String),
    Office(String),
    Site(String),
    Asset(String),
    PersonSlot(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialRelation {
    pub id: String,
    pub subject: ObjectRef,
    pub predicate: Predicate,
    pub object: ObjectRef,
    pub valid: TimeSpan,
    pub role_id: Option<String>,
    pub weight_bps: Option<u16>,
    pub time_cost_bps: Option<u16>,
    pub cause_event_id: Option<String>,
    pub provenance: Provenance,
    pub rule_id: String,
}
string_enum!(PersonSource {
    LineageSimulated,
    LineageProjected,
    Procedural,
    Author
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidenceEpisode {
    pub site_id: String,
    pub valid: TimeSpan,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialAttributeEpisode {
    pub valid: TimeSpan,
    pub attributes: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonRecord {
    pub entity_id: String,
    pub provider_ref: String,
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub parent_refs: Vec<ObjectRef>,
    pub union_refs: Vec<String>,
    pub residence_episodes: Vec<ResidenceEpisode>,
    pub source: PersonSource,
    pub ancestry_profile: String,
    pub social_attribute_episodes: Vec<SocialAttributeEpisode>,
    pub coverage: TimeSpan,
}
impl PersonRecord {
    pub fn is_alive_and_covered(&self, year: i32) -> bool {
        self.coverage.contains(year)
            && self.birth_year <= year
            && self.death_year.is_none_or(|d| year < d)
    }
}

/// Les valeurs numériques sont celles du contrat ; sérialisation JSON entière.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum EventPhase {
    Bootstrap = 0,
    ContextAndShocks = 10,
    Births = 20,
    Deaths = 30,
    DivorcesAndUnions = 35,
    Residences = 40,
    InstitutionsAndVacancies = 50,
    CohesionAndSplits = 55,
    Assignments = 60,
    OtherRelations = 70,
    StatePublication = 80,
}
impl Serialize for EventPhase {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(*self as u8)
    }
}
impl<'de> Deserialize<'de> for EventPhase {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match u8::deserialize(d)? {
            0 => Ok(Self::Bootstrap),
            10 => Ok(Self::ContextAndShocks),
            20 => Ok(Self::Births),
            30 => Ok(Self::Deaths),
            35 => Ok(Self::DivorcesAndUnions),
            40 => Ok(Self::Residences),
            50 => Ok(Self::InstitutionsAndVacancies),
            55 => Ok(Self::CohesionAndSplits),
            60 => Ok(Self::Assignments),
            70 => Ok(Self::OtherRelations),
            80 => Ok(Self::StatePublication),
            _ => Err(serde::de::Error::custom("phase v2 inconnue")),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryEvent {
    pub id: String,
    pub owner_scope: String,
    pub kind: String,
    pub year: i32,
    pub phase: EventPhase,
    pub causes: Vec<String>,
    pub participants: Vec<ObjectRef>,
    pub provenance: Provenance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    ProviderUnavailable,
    ProviderVersionMismatch,
    UnsupportedCapability,
    PopulationConstraintsUnsatisfied,
    ExactImpactInfeasible,
    BoundaryInfeasible,
    InvalidPopulationResult,
    CoreBudgetExceeded,
    BudgetExceeded,
    CanonicalDependencyCycle,
    InvalidAddress,
    InvalidCanonicalValue,
    InvalidAtlas,
    InvalidPopulationPlan,
    InvalidInterval,
    InvalidCausalOrder,
    CompositionCycle,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryError {
    pub code: ErrorCode,
    pub owner: Option<String>,
    pub at_year: Option<i32>,
    pub constraint: String,
    pub plan_id: Option<String>,
}
impl HistoryError {
    pub fn new(code: ErrorCode, constraint: impl Into<String>) -> Self {
        Self {
            code,
            owner: None,
            at_year: None,
            constraint: constraint.into(),
            plan_id: None,
        }
    }
}
impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.constraint)
    }
}
impl std::error::Error for HistoryError {}
