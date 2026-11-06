//! v2 facts, measures and diagnostics (§3–5, §7.3, §13, §27).
use super::canonical::CanonicalRecord;
use super::ids::{AddressKind, OwnerScope, SemanticAddress};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Absolute bound of years and blocks (plan schema, §3.2).
pub const MAX_ABS_YEAR: i32 = 100_000;
/// Absolute bound of integer metric coordinates (plan schema, §4.2).
pub const MAX_ABS_COORDINATE_MM: i64 = 9_000_000_000_000_000;
/// Maximum number of exposed parents: only two-parent reproduction is
/// delivered before S8 (§6.2).
pub const MAX_PARENT_REFS: usize = 2;

macro_rules! string_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
pub(crate) use string_enum;

/// Optional field: omitted or present with a value, never `null`.
pub(crate) fn present_optional<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// Required nullable field: `null` means "unknown" (§4.3), absence of the
/// field is rejected. Without this attribute, serde would read absence as `None`.
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
    /// An empty life interval is allowed; other intervals are strict.
    pub fn validate(self, allow_empty: bool) -> Result<(), HistoryError> {
        if self.start_year < -MAX_ABS_YEAR
            || self.end_year > MAX_ABS_YEAR
            || self.start_year > self.end_year
            || (!allow_empty && self.start_year == self.end_year)
        {
            Err(HistoryError::new(
                ErrorCode::InvalidInterval,
                "interval.invalid",
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
/// Name tree of the Namebench grammar, stored in the canon and rendered as
/// text at display time. Opaque value for Chronos, subject to the canonical
/// check at construction as at reading: no float enters it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Value", into = "Value")]
pub struct NameTree(Value);
impl NameTree {
    pub fn new(value: Value) -> Result<Self, HistoryError> {
        if value.is_null() {
            return Err(HistoryError::new(
                ErrorCode::InvalidCanonicalValue,
                "name_tree.null",
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
    pub semantic_address: SemanticAddress,
    pub kind: String,
    pub archetype_id: String,
    pub nature: EntityNature,
    pub agency: Agency,
    pub owner_scope: OwnerScope,
    pub existence: TimeSpan,
    pub culture_refs: Vec<String>,
    pub institution_ref: Option<String>,
    pub provenance: Provenance,
    /// Absent as long as the name grammar has produced nothing.
    #[serde(
        default,
        deserialize_with = "present_optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub name_tree: Option<NameTree>,
}
fn identity_error(owner: &str, at_year: i32, message: &str) -> HistoryError {
    HistoryError {
        owner: Some(owner.into()),
        at_year: Some(at_year),
        ..HistoryError::new(ErrorCode::InvalidRecordIdentity, message)
    }
}
impl SocialEntity {
    /// `id` decodes to `semantic_address`, the address of an entity, a group
    /// slot, a household or a person, consistent with `kind`, `nature`,
    /// `archetype_id` and `owner_scope`.
    pub fn validate(&self) -> Result<(), HistoryError> {
        let fail = |message: &str| identity_error(&self.id, self.existence.start_year, message);
        let address =
            SemanticAddress::decode(&self.id).map_err(|_| fail("entity.id_not_addressed"))?;
        if address != self.semantic_address {
            return Err(fail("entity.id_differs_from_address"));
        }
        if address.owner().as_ref().map(|o| o.scope()) != Some(&self.owner_scope) {
            return Err(fail("entity.owner_scope_differs"));
        }
        let is_person = self.nature == EntityNature::Person;
        let tail = address.owned_tail();
        let coherent = match address.kind() {
            AddressKind::Entity => !is_person && self.kind == tail[0],
            AddressKind::Group => !is_person && self.archetype_id == tail[1],
            AddressKind::Household => !is_person && self.kind == "household",
            AddressKind::Person => is_person && self.kind == "person",
            _ => false,
        };
        if !coherent {
            return Err(fail("entity.kind_incompatible"));
        }
        // Only a life may be an empty interval (§3.2).
        self.existence
            .validate(is_person)
            .map_err(|e| HistoryError {
                owner: Some(self.id.clone()),
                at_year: Some(self.existence.start_year),
                ..e
            })
    }
}
impl CanonicalRecord for SocialEntity {
    type Key = String;
    fn canonical_key(&self) -> String {
        self.id.clone()
    }
    fn normalize(&mut self) {
        self.culture_refs.sort();
    }
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
    pub conduct_tags: BTreeSet<String>,
    pub visibility: Visibility,
    pub goals: Vec<String>,
    pub assets: Vec<String>,
    pub footprint_refs: Vec<String>,
    pub institution_version: String,
}
impl EntityState {
    /// `conduct_tags` is the catalogue received: it evolves as data (§5.2).
    pub fn validate(&self, conduct_tags: &BTreeSet<String>) -> Result<(), HistoryError> {
        check_conduct_tags(&self.conduct_tags, conduct_tags).map_err(|mut e| {
            e.owner = Some(self.entity_id.clone());
            e.at_year = Some(self.at_year);
            e
        })
    }
}
impl CanonicalRecord for EntityState {
    type Key = (String, i32);
    fn canonical_key(&self) -> Self::Key {
        (self.entity_id.clone(), self.at_year)
    }
    fn normalize(&mut self) {
        self.capabilities.sort();
        self.goals.sort();
        self.assets.sort();
        self.footprint_refs.sort();
    }
}
fn check_conduct_tags(
    tags: &BTreeSet<String>,
    catalogue: &BTreeSet<String>,
) -> Result<(), HistoryError> {
    if tags.is_subset(catalogue) {
        Ok(())
    } else {
        Err(HistoryError::new(
            ErrorCode::InvalidAlignment,
            "entity.conduct_tag_not_in_catalogue",
        ))
    }
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
impl ObjectRef {
    /// Canonical form: an address type gives a single variant. A person is a
    /// `PersonSlot`, an entity of type `office` an `Office`.
    pub fn from_address(address: &SemanticAddress) -> Result<Self, HistoryError> {
        let id = address.encode();
        match address.kind() {
            AddressKind::Entity if address.owned_tail()[0] == "office" => Ok(Self::Office(id)),
            AddressKind::Entity | AddressKind::Group | AddressKind::Household => {
                Ok(Self::Entity(id))
            }
            AddressKind::Person => Ok(Self::PersonSlot(id)),
            AddressKind::Site => Ok(Self::Site(id)),
            AddressKind::Asset => Ok(Self::Asset(id)),
            AddressKind::Activity => Ok(Self::Activity(id)),
            _ => Err(HistoryError::new(
                ErrorCode::InvalidRecordIdentity,
                "object_ref.address_not_referable",
            )),
        }
    }
    /// Designated address, rejected if the variant is not its canonical form.
    pub fn address(&self) -> Result<SemanticAddress, HistoryError> {
        let id = match self {
            Self::Entity(id)
            | Self::Office(id)
            | Self::Site(id)
            | Self::Asset(id)
            | Self::PersonSlot(id)
            | Self::Activity(id) => id,
        };
        let address = SemanticAddress::decode(id)?;
        if &Self::from_address(&address)? != self {
            return Err(HistoryError::new(
                ErrorCode::InvalidRecordIdentity,
                "object_ref.variant_not_canonical",
            ));
        }
        Ok(address)
    }
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
impl SocialRelation {
    /// `id` decodes to a relation address carrying the same predicate, endpoints
    /// and start as the record.
    pub fn validate(&self) -> Result<(), HistoryError> {
        let fail = |message: &str| identity_error(&self.id, self.valid.start_year, message);
        let address =
            SemanticAddress::decode(&self.id).map_err(|_| fail("entity.id_not_addressed"))?;
        if address.kind() != AddressKind::Relation {
            return Err(fail("relation.id_not_relation"));
        }
        let link = address.link().ok_or_else(|| fail("relation.unreadable"))?;
        if link.predicate != Some(self.predicate)
            || ObjectRef::from_address(&link.subject).as_ref() != Ok(&self.subject)
            || ObjectRef::from_address(&link.object).as_ref() != Ok(&self.object)
            || link.start != self.valid.start_year
        {
            return Err(fail("relation.differs_from_address"));
        }
        Ok(())
    }
}
impl CanonicalRecord for SocialRelation {
    type Key = String;
    fn canonical_key(&self) -> String {
        self.id.clone()
    }
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
/// Parent of a table row: another row, or a bounded and typed outside
/// reference (§24.3). An outside parent is never an invented person.
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
    /// Years the provider observed for this person. Unrelated to the `coverage`
    /// axis of a measure (§4.3).
    pub observed_span: TimeSpan,
    pub alignment_episodes: Vec<AlignmentEpisode>,
}
impl PersonRecord {
    pub fn is_alive_and_covered(&self, year: i32) -> bool {
        self.observed_span.contains(year)
            && self.birth_year <= year
            && self.death_year.is_none_or(|d| year < d)
    }
    /// `entity_id` is the address of this person, and its local ID `provider_ref`.
    /// Bounded dates and parent references, ordered alignment episodes: an episode
    /// after the first cites the event that changes the axes (§27.5).
    pub fn validate(&self, conduct_tags: &BTreeSet<String>) -> Result<(), HistoryError> {
        let fail = |code, message: &str| HistoryError {
            owner: Some(self.entity_id.clone()),
            at_year: Some(self.birth_year),
            ..HistoryError::new(code, message)
        };
        let identity = ErrorCode::InvalidRecordIdentity;
        let address = SemanticAddress::decode(&self.entity_id)
            .map_err(|_| fail(identity, "person_table.entity_id_not_addressed"))?;
        if address.kind() != AddressKind::Person || address.owned_tail()[1] != self.provider_ref {
            return Err(fail(identity, "person_table.entity_id_wrong_type"));
        }
        let years = -MAX_ABS_YEAR..=MAX_ABS_YEAR;
        let parents: BTreeSet<_> = self.parent_refs.iter().map(ParentRef::id).collect();
        if !years.contains(&self.birth_year)
            || self
                .death_year
                .is_some_and(|y| !years.contains(&y) || y < self.birth_year)
            || self.parent_refs.len() > MAX_PARENT_REFS
            || parents.len() != self.parent_refs.len()
            || !parents.iter().all(|id| super::atlas::valid_id(id))
        {
            return Err(fail(
                ErrorCode::InvalidCanonicalValue,
                "person_table.life_out_of_bounds",
            ));
        }
        for parent in &self.parent_refs {
            if let ParentRef::Slot { person_id } = parent {
                if SemanticAddress::decode(person_id).map(|a| a.kind()) != Ok(AddressKind::Person) {
                    return Err(fail(identity, "person_table.parent_not_addressed"));
                }
            }
        }
        self.observed_span
            .validate(true)
            .map_err(|e| fail(e.code, &e.constraint))?;
        let mut previous_end = None;
        for episode in &self.alignment_episodes {
            episode
                .valid
                .validate(false)
                .map_err(|e| fail(e.code, &e.constraint))?;
            check_conduct_tags(&episode.conduct_tags, conduct_tags)
                .map_err(|e| fail(e.code, &e.constraint))?;
            if previous_end.is_some_and(|end| episode.valid.start_year < end) {
                return Err(fail(
                    ErrorCode::InvalidAlignment,
                    "alignment.episodes_overlap",
                ));
            }
            if previous_end.is_some() && episode.cause_event_id.is_none() {
                return Err(fail(
                    ErrorCode::InvalidAlignment,
                    "alignment.change_without_event",
                ));
            }
            previous_end = Some(episode.valid.end_year);
        }
        Ok(())
    }
}

impl CanonicalRecord for PersonRecord {
    type Key = String;
    fn canonical_key(&self) -> String {
        self.entity_id.clone()
    }
    fn normalize(&mut self) {
        self.parent_refs.sort_by(|a, b| a.id().cmp(b.id()));
        self.union_refs.sort();
    }
}

/// Conduct axes (§27.1), bounded at construction as at reading.
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
                "alignment.axis_out_of_range",
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
/// Thresholds of §27.2. Their values come from `engine_defaults.alignment` (S1);
/// the S0 core fixes none of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConductThresholds {
    /// `lawless` requires `morality ≤` this value…
    pub lawless_morality_max_bps: i32,
    /// …and `lawfulness ≤` this one.
    pub lawless_lawfulness_max_bps: i32,
    /// `crooked`: `morality ≤` this value.
    pub crooked_morality_max_bps: i32,
    /// `ordinary`: `morality <` this value.
    pub ordinary_morality_below_bps: i32,
    /// `upright`: `morality <` this value; `saintly` beyond.
    pub upright_morality_below_bps: i32,
}
/// Pure function: the class is never stored nor drawn separately (§27.2). The
/// first rule that holds applies.
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
/// A category or a population has no conduct class (§27.2).
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
string_enum!(Visibility { Open, Covert });
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignmentEpisode {
    pub valid: TimeSpan,
    pub axes: AlignmentAxes,
    pub conduct_tags: BTreeSet<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub cause_event_id: Option<String>,
    pub provenance: Provenance,
}
string_enum!(LegalStatus {
    Licit,
    Tolerated,
    Proscribed
});
/// Legality relative to a jurisdiction and dated (§27.3), distinct from a free
/// relation and independent of the conduct class.
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
    /// End event of a segment without an immediate successor.
    #[serde(deserialize_with = "required_nullable")]
    pub end_cause_event_id: Option<String>,
    pub provenance: Provenance,
    pub rule_id: String,
}
impl LegalStanding {
    /// `id` decodes to a standing address carrying the same subject, the same
    /// jurisdiction and the same start.
    pub fn validate(&self) -> Result<(), HistoryError> {
        let fail = |message: &str| identity_error(&self.id, self.valid.start_year, message);
        let address =
            SemanticAddress::decode(&self.id).map_err(|_| fail("entity.id_not_addressed"))?;
        if address.kind() != AddressKind::Standing {
            return Err(fail("legal.id_not_legal_standing"));
        }
        let link = address.link().ok_or_else(|| fail("legal.unreadable"))?;
        if ObjectRef::from_address(&link.subject).as_ref() != Ok(&self.subject)
            || ObjectRef::from_address(&link.object).as_ref() != Ok(&self.jurisdiction)
            || link.start != self.valid.start_year
        {
            return Err(fail("legal.differs_from_address"));
        }
        Ok(())
    }
}
impl CanonicalRecord for LegalStanding {
    type Key = String;
    fn canonical_key(&self) -> String {
        self.id.clone()
    }
}

/// Numeric values are those of the contract; integer JSON serialization.
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
            _ => Err(serde::de::Error::custom("event.unknown_phase")),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryEvent {
    pub id: String,
    pub owner_scope: OwnerScope,
    pub kind: String,
    pub year: i32,
    /// Exclusive end of a frame event; `null` for a dated event.
    #[serde(deserialize_with = "required_nullable")]
    pub end_year: Option<i32>,
    pub phase: EventPhase,
    pub causes: Vec<String>,
    pub participants: Vec<ObjectRef>,
    pub provenance: Provenance,
}
impl HistoryEvent {
    /// `id` decodes to an event address of this type and this owner.
    pub fn validate(&self) -> Result<(), HistoryError> {
        let fail = |message: &str| identity_error(&self.id, self.year, message);
        let address =
            SemanticAddress::decode(&self.id).map_err(|_| fail("entity.id_not_addressed"))?;
        if address.kind() != AddressKind::Event
            || address.owner().as_ref().map(|o| o.scope()) != Some(&self.owner_scope)
            || address.owned_tail()[0] != self.kind
        {
            return Err(fail("event.differs_from_address"));
        }
        Ok(())
    }
}
impl CanonicalRecord for HistoryEvent {
    type Key = String;
    fn canonical_key(&self) -> String {
        self.id.clone()
    }
    fn normalize(&mut self) {
        self.causes.sort();
        self.participants.sort();
    }
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
    InvalidAlignment,
    InvalidLegalStanding,
    InvalidRecordIdentity,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryError {
    pub code: ErrorCode,
    pub owner: Option<String>,
    pub at_year: Option<i32>,
    pub constraint: String,
    pub plan_id: Option<String>,
    /// Values substituted into the translated message, by placeholder name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
}
impl HistoryError {
    pub fn new(code: ErrorCode, constraint: impl Into<String>) -> Self {
        Self {
            code,
            owner: None,
            at_year: None,
            constraint: constraint.into(),
            plan_id: None,
            params: BTreeMap::new(),
        }
    }
    pub fn with_param(mut self, name: &str, value: impl Into<String>) -> Self {
        self.params.insert(name.into(), value.into());
        self
    }
}
impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.constraint)
    }
}
impl std::error::Error for HistoryError {}
