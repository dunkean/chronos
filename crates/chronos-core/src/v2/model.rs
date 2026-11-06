//! Faits, mesures et diagnostics v2 (§3–5, §7.3, §13, §27).
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Borne absolue des années et blocs (schéma de plan, §3.2).
pub const MAX_ABS_YEAR: i32 = 100_000;
/// Borne absolue des coordonnées métriques entières (schéma de plan, §4.2).
pub const MAX_ABS_COORDINATE_MM: i64 = 9_000_000_000_000_000;
/// Nombre maximal de parents exposés : seule la reproduction biparentale est
/// livrée avant S8 (§6.2).
pub const MAX_PARENT_REFS: usize = 2;

macro_rules! string_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
pub(crate) use string_enum;

/// Champ facultatif : omis ou présent avec une valeur, jamais `null`.
pub(crate) fn present_optional<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// Champ nullable obligatoire : `null` dit « inconnu » (§4.3), l'absence du
/// champ est refusée. Sans cet attribut, serde lirait l'absence comme `None`.
pub(crate) fn required_nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

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
        if self.start_year < -MAX_ABS_YEAR
            || self.end_year > MAX_ABS_YEAR
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
    #[serde(deserialize_with = "required_nullable")]
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
/// Arbre de nom de la grammaire Namebench, stocké dans le canon et rendu en
/// texte à l'affichage. Valeur opaque pour Chronos, soumise au contrôle
/// canonique à la construction comme à la lecture : aucun flottant n'y entre.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Value", into = "Value")]
pub struct NameTree(Value);
impl NameTree {
    pub fn new(value: Value) -> Result<Self, HistoryError> {
        if value.is_null() {
            return Err(HistoryError::new(
                ErrorCode::InvalidCanonicalValue,
                "arbre de nom nul : omettre le champ",
            ));
        }
        super::canonical::canonical_bytes(&value)?;
        Ok(Self(value))
    }
    pub fn value(&self) -> &Value {
        &self.0
    }
}
impl TryFrom<Value> for NameTree {
    type Error = HistoryError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<NameTree> for Value {
    fn from(tree: NameTree) -> Self {
        tree.0
    }
}
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
    /// Absent tant que la grammaire de noms n'a rien produit.
    #[serde(
        default,
        deserialize_with = "present_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub name_tree: Option<NameTree>,
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
    pub alignment: AlignmentAxes,
    pub conduct_tags: BTreeSet<ConductTag>,
    pub visibility: Visibility,
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
    Activity(String),
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
string_enum!(ExternalParentOrigin {
    CoverageLimit,
    ExternalArrival
});
/// Parent d'une ligne de table : autre ligne, ou référence extérieure bornée
/// et typée (§24.3). Un parent extérieur n'est jamais une personne inventée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParentRef {
    Slot {
        person_id: String,
    },
    External {
        origin: ExternalParentOrigin,
        ref_id: String,
    },
}
impl ParentRef {
    fn id(&self) -> &str {
        match self {
            Self::Slot { person_id } => person_id,
            Self::External { ref_id, .. } => ref_id,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonRecord {
    pub entity_id: String,
    pub provider_ref: String,
    pub birth_year: i32,
    #[serde(deserialize_with = "required_nullable")]
    pub death_year: Option<i32>,
    pub parent_refs: Vec<ParentRef>,
    pub union_refs: Vec<String>,
    pub residence_episodes: Vec<ResidenceEpisode>,
    pub source: PersonSource,
    pub ancestry_profile: String,
    pub social_attribute_episodes: Vec<SocialAttributeEpisode>,
    /// Années que le fournisseur a observées pour cette personne. Rien à voir
    /// avec l'axe `coverage` d'une mesure (§4.3).
    pub observed_span: TimeSpan,
    pub alignment_episodes: Vec<AlignmentEpisode>,
}
impl PersonRecord {
    pub fn is_alive_and_covered(&self, year: i32) -> bool {
        self.observed_span.contains(year)
            && self.birth_year <= year
            && self.death_year.is_none_or(|d| year < d)
    }
    /// Bornes des références parentales et épisodes d'alignement ordonnés. Un
    /// épisode après le premier cite l'événement qui change les axes (§27.5).
    pub fn validate(&self) -> Result<(), HistoryError> {
        let fail = |code, message: &str| HistoryError {
            owner: Some(self.entity_id.clone()),
            at_year: Some(self.birth_year),
            ..HistoryError::new(code, message)
        };
        let years = -MAX_ABS_YEAR..=MAX_ABS_YEAR;
        let parents: BTreeSet<_> = self.parent_refs.iter().map(ParentRef::id).collect();
        if !years.contains(&self.birth_year)
            || self.death_year.is_some_and(|y| !years.contains(&y))
            || self.parent_refs.len() > MAX_PARENT_REFS
            || parents.len() != self.parent_refs.len()
            || !parents.iter().all(|id| super::atlas::valid_id(id))
        {
            return Err(fail(
                ErrorCode::InvalidCanonicalValue,
                "dates de vie ou références parentales hors bornes",
            ));
        }
        let mut previous_end = None;
        for episode in &self.alignment_episodes {
            episode
                .valid
                .validate(false)
                .map_err(|e| fail(e.code, &e.constraint))?;
            if previous_end.is_some_and(|end| episode.valid.start_year < end) {
                return Err(fail(
                    ErrorCode::InvalidAlignment,
                    "épisodes d'alignement non ordonnés ou chevauchants",
                ));
            }
            if previous_end.is_some() && episode.cause_event_id.is_none() {
                return Err(fail(
                    ErrorCode::InvalidAlignment,
                    "changement d'alignement sans événement",
                ));
            }
            previous_end = Some(episode.valid.end_year);
        }
        Ok(())
    }
}

/// Axes de conduite (§27.1), bornés à la construction comme à la lecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AlignmentAxesWire", into = "AlignmentAxesWire")]
pub struct AlignmentAxes {
    morality_bps: i32,
    lawfulness_bps: i32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlignmentAxesWire {
    morality_bps: i32,
    lawfulness_bps: i32,
}
impl From<AlignmentAxes> for AlignmentAxesWire {
    fn from(a: AlignmentAxes) -> Self {
        Self {
            morality_bps: a.morality_bps,
            lawfulness_bps: a.lawfulness_bps,
        }
    }
}
impl TryFrom<AlignmentAxesWire> for AlignmentAxes {
    type Error = HistoryError;
    fn try_from(a: AlignmentAxesWire) -> Result<Self, Self::Error> {
        Self::new(a.morality_bps, a.lawfulness_bps)
    }
}
impl AlignmentAxes {
    pub const MAX_ABS_BPS: i32 = 10_000;
    pub fn new(morality_bps: i32, lawfulness_bps: i32) -> Result<Self, HistoryError> {
        let bound = -Self::MAX_ABS_BPS..=Self::MAX_ABS_BPS;
        if !bound.contains(&morality_bps) || !bound.contains(&lawfulness_bps) {
            return Err(HistoryError::new(
                ErrorCode::InvalidAlignment,
                "axe d'alignement hors de ±10 000",
            ));
        }
        Ok(Self {
            morality_bps,
            lawfulness_bps,
        })
    }
    pub fn morality_bps(self) -> i32 {
        self.morality_bps
    }
    pub fn lawfulness_bps(self) -> i32 {
        self.lawfulness_bps
    }
}
string_enum!(ConductClass {
    Lawless,
    Crooked,
    Ordinary,
    Upright,
    Saintly
});
/// Seuils du §27.2. Leurs valeurs viennent de `engine_defaults.alignment` (S1) ;
/// le noyau S0 n'en fige aucune.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConductThresholds {
    /// `lawless` exige `morality ≤` cette valeur…
    pub lawless_morality_max_bps: i32,
    /// …et `lawfulness ≤` celle-ci.
    pub lawless_lawfulness_max_bps: i32,
    /// `crooked` : `morality ≤` cette valeur.
    pub crooked_morality_max_bps: i32,
    /// `ordinary` : `morality <` cette valeur.
    pub ordinary_morality_below_bps: i32,
    /// `upright` : `morality <` cette valeur ; `saintly` au-delà.
    pub upright_morality_below_bps: i32,
}
/// Fonction pure : la classe n'est jamais stockée ni tirée à part (§27.2). La
/// première règle vérifiée s'applique.
pub fn conduct_class(axes: AlignmentAxes, thresholds: &ConductThresholds) -> ConductClass {
    let morality = axes.morality_bps;
    if morality <= thresholds.lawless_morality_max_bps
        && axes.lawfulness_bps <= thresholds.lawless_lawfulness_max_bps
    {
        ConductClass::Lawless
    } else if morality <= thresholds.crooked_morality_max_bps {
        ConductClass::Crooked
    } else if morality < thresholds.ordinary_morality_below_bps {
        ConductClass::Ordinary
    } else if morality < thresholds.upright_morality_below_bps {
        ConductClass::Upright
    } else {
        ConductClass::Saintly
    }
}
/// Une catégorie ou une population n'a pas de classe de conduite (§27.2).
pub fn entity_conduct_class(
    nature: EntityNature,
    axes: AlignmentAxes,
    thresholds: &ConductThresholds,
) -> Option<ConductClass> {
    match nature {
        EntityNature::Person | EntityNature::OrganizedGroup | EntityNature::Community => {
            Some(conduct_class(axes, thresholds))
        }
        EntityNature::Category | EntityNature::Population => None,
    }
}
// Variantes en ordre alphabétique : `BTreeSet` sérialise ainsi la liste dans
// l'ordre canonique par ID (§4.2). Le catalogue fermé (§27.4) n'expose que les
// tags que la spécification nomme ; S1 y ajoute les siens.
string_enum!(ConductTag {
    Blasphemy,
    Charity,
    Extortion,
    Sorcery,
    Usury,
    Violence
});
string_enum!(Visibility { Open, Covert });
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignmentEpisode {
    pub valid: TimeSpan,
    pub axes: AlignmentAxes,
    pub conduct_tags: BTreeSet<ConductTag>,
    #[serde(deserialize_with = "required_nullable")]
    pub cause_event_id: Option<String>,
    pub provenance: Provenance,
}
string_enum!(LegalStatus {
    Licit,
    Tolerated,
    Proscribed
});
/// Légalité relative à une juridiction et datée (§27.3), distincte d'une
/// relation libre et indépendante de la classe de conduite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegalStanding {
    pub id: String,
    pub subject: ObjectRef,
    pub jurisdiction: ObjectRef,
    pub status: LegalStatus,
    pub valid: TimeSpan,
    #[serde(deserialize_with = "required_nullable")]
    pub cause_event_id: Option<String>,
    pub provenance: Provenance,
    pub rule_id: String,
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
    /// Fin exclusive d'un événement-cadre ; `null` pour un événement daté.
    #[serde(deserialize_with = "required_nullable")]
    pub end_year: Option<i32>,
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
    InvalidImpactContract,
    IncompleteCatalogue,
    InvalidAlignment,
    InvalidLegalStanding,
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
