//! Complete addresses and indexed draws; no hash serves as identity.
//!
//! Each object has a single address form: `from_fields` fixes, per type, the
//! number and role of its fields. An owner or a domain always enters the
//! address flat, through `Owner` or `DomainKey`, never through a joined string.
use super::model::{string_enum, ErrorCode, HistoryError, ObjectRef, Predicate, MAX_ABS_YEAR};
use serde::{Deserialize, Serialize};

pub const MAX_ID_CHARS: usize = 4096;
/// Start of the base64url form of the `chr2\0` signature: starting point for
/// the search of an encoded ID or payload inside a field. Its presence alone is not enough.
const ENCODED_SIGNATURE: &str = "Y2hyMg";
const OWNER_WORLD: &str = "world";
const OWNER_CELL: &str = "cell";
const OWNER_SITE: &str = "site";
const OWNER_DOMAIN: &str = "domain";

string_enum!(AddressKind {
    Entity,
    Person,
    Tenure,
    Event,
    Impact,
    Effect,
    Episode,
    Group,
    Household,
    Relation,
    Standing,
    Transfer,
    Plan,
    Domain,
    Site,
    Asset,
    Activity
});
impl AddressKind {
    const ALL: [Self; 17] = [
        Self::Entity,
        Self::Person,
        Self::Tenure,
        Self::Event,
        Self::Impact,
        Self::Effect,
        Self::Episode,
        Self::Group,
        Self::Household,
        Self::Relation,
        Self::Standing,
        Self::Transfer,
        Self::Plan,
        Self::Domain,
        Self::Site,
        Self::Asset,
        Self::Activity,
    ];
    fn prefix(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Person => "person",
            Self::Tenure => "tenure",
            Self::Event => "event",
            Self::Impact => "impact",
            Self::Effect => "effect",
            Self::Episode => "episode",
            Self::Group => "group",
            Self::Household => "household",
            Self::Relation => "relation",
            Self::Standing => "standing",
            Self::Transfer => "transfer",
            Self::Plan => "plan",
            Self::Domain => "domain",
            Self::Site => "site",
            Self::Asset => "asset",
            Self::Activity => "activity",
        }
    }
    fn from_prefix(prefix: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.prefix() == prefix)
    }
    /// Endpoint of a relation or a standing, or target of an effect: only the
    /// types an `ObjectRef` can designate.
    fn from_endpoint_prefix(prefix: &str) -> Result<Self, HistoryError> {
        Self::from_prefix(prefix)
            .filter(|k| {
                matches!(
                    k,
                    Self::Entity
                        | Self::Group
                        | Self::Household
                        | Self::Person
                        | Self::Site
                        | Self::Asset
                        | Self::Activity
                )
            })
            .ok_or_else(|| invalid("id.bad_endpoint_type"))
    }
}

/// Starting decade block (§3.2): Euclidean division, including before year
/// zero. `decade_block(-1) == -10`.
pub fn decade_block(year: i32) -> i32 {
    year.div_euclid(10) * 10
}

/// Demographic domain: world, region, social profile and ordinal (§3.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DomainKey {
    world: String,
    region: String,
    society: String,
    ordinal: u32,
}
impl DomainKey {
    pub fn new(
        world: &str,
        region: &str,
        society: &str,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        let key = Self {
            world: world.into(),
            region: region.into(),
            society: society.into(),
            ordinal,
        };
        SemanticAddress::domain(&key)?;
        Ok(key)
    }
    /// Reads back an encoded `domain_id`. An address of another type is rejected.
    pub fn from_id(id: &str) -> Result<Self, HistoryError> {
        let address = SemanticAddress::decode(id)?;
        if address.kind != AddressKind::Domain {
            return Err(invalid("id.domain_address_required"));
        }
        let ordinal = address.fields[3]
            .parse()
            .map_err(|_| invalid("id.ordinal_out_of_range"))?;
        Self::new(
            &address.fields[0],
            &address.fields[1],
            &address.fields[2],
            ordinal,
        )
    }
    pub fn world(&self) -> &str {
        &self.world
    }
    pub fn region(&self) -> &str {
        &self.region
    }
    pub fn society(&self) -> &str {
        &self.society
    }
    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }
    fn flat_fields(&self) -> Vec<String> {
        vec![
            self.world.clone(),
            self.region.clone(),
            self.society.clone(),
            self.ordinal.to_string(),
        ]
    }
}

/// Scope of an owner, without the world: the atlas carries it too, and it is
/// an input of the canon, hence earlier than any `canon_id` (§4.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerScope {
    World,
    Cell {
        cell_id: String,
    },
    Site {
        site_id: String,
    },
    Domain {
        region: String,
        society: String,
        ordinal: u32,
    },
}
impl OwnerScope {
    pub fn cell(cell_id: &str) -> Self {
        Self::Cell {
            cell_id: cell_id.into(),
        }
    }
    pub fn site(site_id: &str) -> Self {
        Self::Site {
            site_id: site_id.into(),
        }
    }
}
/// Diagnostic label, never an identity.
impl std::fmt::Display for OwnerScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::World => f.write_str(OWNER_WORLD),
            Self::Cell { cell_id } => f.write_str(cell_id),
            Self::Site { site_id } => f.write_str(site_id),
            Self::Domain {
                region,
                society,
                ordinal,
            } => write!(f, "{region}/{society}/{ordinal}"),
        }
    }
}
/// Complete owner of an object: world and scope. The scope type is part of
/// the address: a cell and a domain remain two distinct owners, even if the
/// cell's name imitated the shape of a domain.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Owner {
    world: String,
    scope: OwnerScope,
}
impl Owner {
    pub fn new(world: &str, scope: OwnerScope) -> Self {
        Self {
            world: world.into(),
            scope,
        }
    }
    pub fn whole_world(world: &str) -> Self {
        Self::new(world, OwnerScope::World)
    }
    pub fn cell(world: &str, cell: &str) -> Self {
        Self::new(world, OwnerScope::cell(cell))
    }
    pub fn site(world: &str, site: &str) -> Self {
        Self::new(world, OwnerScope::site(site))
    }
    pub fn domain(key: &DomainKey) -> Self {
        Self::new(
            &key.world,
            OwnerScope::Domain {
                region: key.region.clone(),
                society: key.society.clone(),
                ordinal: key.ordinal,
            },
        )
    }
    pub fn world(&self) -> &str {
        &self.world
    }
    pub fn scope(&self) -> &OwnerScope {
        &self.scope
    }
    fn flat_fields(&self) -> Vec<String> {
        let mut fields = vec![self.world.clone()];
        match &self.scope {
            OwnerScope::World => fields.push(OWNER_WORLD.into()),
            OwnerScope::Cell { cell_id } => fields.extend([OWNER_CELL.into(), cell_id.clone()]),
            OwnerScope::Site { site_id } => fields.extend([OWNER_SITE.into(), site_id.clone()]),
            OwnerScope::Domain {
                region,
                society,
                ordinal,
            } => fields.extend([
                OWNER_DOMAIN.into(),
                region.clone(),
                society.clone(),
                ordinal.to_string(),
            ]),
        }
        fields
    }
    /// Reads back the owner of an address whose shape is already checked.
    fn from_flat_fields(fields: &[String]) -> Option<Self> {
        let scope = match fields.get(1)?.as_str() {
            OWNER_WORLD => OwnerScope::World,
            OWNER_CELL => OwnerScope::cell(fields.get(2)?),
            OWNER_SITE => OwnerScope::site(fields.get(2)?),
            OWNER_DOMAIN => OwnerScope::Domain {
                region: fields.get(2)?.clone(),
                society: fields.get(3)?.clone(),
                ordinal: fields.get(4)?.parse().ok()?,
            },
            _ => return None,
        };
        Some(Self::new(fields.first()?, scope))
    }
}

/// Reading of a relation or legal standing address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkParts {
    pub owner: Owner,
    pub predicate: Option<Predicate>,
    pub subject: SemanticAddress,
    pub object: SemanticAddress,
    pub start: i32,
    pub ordinal: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AddressWire", into = "AddressWire")]
pub struct SemanticAddress {
    kind: AddressKind,
    fields: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AddressWire {
    kind: AddressKind,
    fields: Vec<String>,
}
impl From<SemanticAddress> for AddressWire {
    fn from(a: SemanticAddress) -> Self {
        Self {
            kind: a.kind,
            fields: a.fields,
        }
    }
}
impl TryFrom<AddressWire> for SemanticAddress {
    type Error = HistoryError;
    fn try_from(a: AddressWire) -> Result<Self, Self::Error> {
        Self::from_fields(a.kind, a.fields)
    }
}
fn invalid(message: &str) -> HistoryError {
    HistoryError::new(ErrorCode::InvalidAddress, message)
}

/// Role of a field in the single form of an address.
#[derive(Clone, Copy)]
enum Slot {
    Text,
    Literal(&'static str),
    /// Start year of a decade block.
    Block,
    Year,
    Ordinal,
}
use Slot::{Block, Literal, Ordinal, Text, Year};
const DOMAIN_SLOTS: &[Slot] = &[Text, Text, Text, Ordinal];
fn owner_slots(fields: &[String]) -> Result<&'static [Slot], HistoryError> {
    match fields.get(1).map(String::as_str) {
        Some(OWNER_WORLD) => Ok(&[Text, Literal(OWNER_WORLD)]),
        Some(OWNER_CELL) => Ok(&[Text, Literal(OWNER_CELL), Text]),
        Some(OWNER_SITE) => Ok(&[Text, Literal(OWNER_SITE), Text]),
        Some(OWNER_DOMAIN) => Ok(&[Text, Literal(OWNER_DOMAIN), Text, Text, Ordinal]),
        _ => Err(invalid("id.unknown_owner_type")),
    }
}
fn bounded_year(field: &str) -> Result<i64, HistoryError> {
    let n: i64 = field.parse().map_err(|_| invalid("id.bad_integer"))?;
    if n.to_string() != field {
        return Err(invalid("id.integer_not_canonical"));
    }
    if !(-i64::from(MAX_ABS_YEAR)..=i64::from(MAX_ABS_YEAR)).contains(&n) {
        return Err(invalid("id.year_out_of_range"));
    }
    Ok(n)
}
fn check_slot(field: &str, slot: Slot) -> Result<(), HistoryError> {
    match slot {
        Text => {}
        Literal(value) if field == value => {}
        Literal(_) => return Err(invalid("id.unexpected_literal")),
        Year => {
            bounded_year(field)?;
        }
        Block => {
            if bounded_year(field)?.rem_euclid(10) != 0 {
                return Err(invalid("id.origin_block_misaligned"));
            }
        }
        Ordinal => {
            let n: u32 = field
                .parse()
                .map_err(|_| invalid("id.ordinal_out_of_range"))?;
            if n.to_string() != field {
                return Err(invalid("id.integer_not_canonical"));
            }
        }
    }
    Ok(())
}
fn check_slots(fields: &[String], slots: &[Slot]) -> Result<(), HistoryError> {
    if fields.len() != slots.len() {
        return Err(invalid("id.bad_field_count"));
    }
    fields
        .iter()
        .zip(slots)
        .try_for_each(|(field, slot)| check_slot(field, *slot))
}
/// Form of the types with a fixed number of fields, owner included.
fn fixed_slots(kind: AddressKind, fields: &[String]) -> Result<Vec<Slot>, HistoryError> {
    let (head, tail): (&[Slot], &[Slot]) = match kind {
        AddressKind::Entity => (owner_slots(fields)?, &[Text, Block, Text]),
        AddressKind::Event => (owner_slots(fields)?, &[Text, Block, Ordinal]),
        AddressKind::Impact => (owner_slots(fields)?, &[Text, Block, Ordinal, Text]),
        AddressKind::Asset => (owner_slots(fields)?, &[Literal("asset"), Text, Block, Text]),
        AddressKind::Group => (
            owner_slots(fields)?,
            &[Literal("group_slot"), Text, Block, Ordinal],
        ),
        AddressKind::Transfer => (
            owner_slots(fields)?,
            &[Literal("transfer"), Text, Text, Year, Ordinal],
        ),
        // The office of a tenure is an entity of type `office`, flat.
        AddressKind::Tenure => (
            owner_slots(fields)?,
            &[Literal("office"), Block, Text, Ordinal],
        ),
        AddressKind::Person => (DOMAIN_SLOTS, &[Text, Text]),
        AddressKind::Household => (DOMAIN_SLOTS, &[Literal("household"), Text, Year]),
        AddressKind::Plan | AddressKind::Domain => (DOMAIN_SLOTS, &[]),
        AddressKind::Site | AddressKind::Activity => (&[Text, Text], &[]),
        AddressKind::Relation
        | AddressKind::Standing
        | AddressKind::Effect
        | AddressKind::Episode => return Err(invalid("address.segmented_form_has_no_fixed_slots")),
    };
    Ok([head, tail].concat())
}
fn predicate_name(predicate: Predicate) -> Result<String, HistoryError> {
    serde_json::to_value(predicate)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| invalid("id.predicate_not_serializable"))
}

impl SemanticAddress {
    /// Single entry point, including from `decode` and serde. Printable ASCII
    /// fields without spaces; no encoded ID or payload, at any position.
    pub fn from_fields(kind: AddressKind, fields: Vec<String>) -> Result<Self, HistoryError> {
        check_structure(kind, &fields)?;
        if fields.iter().any(|f| embeds_address(f)) {
            return Err(invalid("id.encoded_parent"));
        }
        let address = Self { kind, fields };
        if address.encoded_len() > MAX_ID_CHARS {
            return Err(invalid("id.too_long"));
        }
        Ok(address)
    }
    pub fn kind(&self) -> AddressKind {
        self.kind
    }
    pub fn fields(&self) -> &[String] {
        &self.fields
    }
    fn owned(
        kind: AddressKind,
        owner: &Owner,
        tail: impl IntoIterator<Item = String>,
    ) -> Result<Self, HistoryError> {
        let mut fields = owner.flat_fields();
        fields.extend(tail);
        Self::from_fields(kind, fields)
    }
    fn in_domain(
        kind: AddressKind,
        domain: &DomainKey,
        tail: impl IntoIterator<Item = String>,
    ) -> Result<Self, HistoryError> {
        let mut fields = domain.flat_fields();
        fields.extend(tail);
        Self::from_fields(kind, fields)
    }
    pub fn entity(owner: &Owner, kind: &str, block: i32, slot: &str) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Entity,
            owner,
            [kind.into(), block.to_string(), slot.into()],
        )
    }
    pub fn person(domain: &DomainKey, algorithm: &str, local: &str) -> Result<Self, HistoryError> {
        Self::in_domain(
            AddressKind::Person,
            domain,
            [algorithm.into(), local.into()],
        )
    }
    /// `office` is the address of an entity of type `office`: its fields are
    /// taken flat, never its encoded ID.
    pub fn tenure(office: &Self, ordinal: u32) -> Result<Self, HistoryError> {
        if office.kind != AddressKind::Entity {
            return Err(invalid("id.office_address_required"));
        }
        let mut fields = office.fields.clone();
        fields.push(ordinal.to_string());
        Self::from_fields(AddressKind::Tenure, fields)
    }
    pub fn event(
        owner: &Owner,
        kind: &str,
        block: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Event,
            owner,
            [kind.into(), block.to_string(), ordinal.to_string()],
        )
    }
    /// Impact contract, child of its parent event.
    pub fn impact(event: &Self, slot: &str) -> Result<Self, HistoryError> {
        if event.kind != AddressKind::Event {
            return Err(invalid("id.event_address_required"));
        }
        let mut fields = event.fields.clone();
        fields.push(slot.into());
        Self::from_fields(AddressKind::Impact, fields)
    }
    /// Exposure of a target to a contract: child of the contract, unique per target.
    /// The target enters flat, without its world. No annual dimension: the local
    /// episodes of the timing profile (§7.2) are its sub-slots.
    pub fn effect(impact: &Self, target: &Self) -> Result<Self, HistoryError> {
        if impact.kind != AddressKind::Impact {
            return Err(invalid("id.impact_address_required"));
        }
        if target.fields[0] != impact.fields[0] {
            return Err(invalid("id.foreign_canon_target"));
        }
        let mut fields = impact.fields.clone();
        fields.push(target.kind.prefix().into());
        fields.extend_from_slice(&target.fields[1..]);
        Self::from_fields(AddressKind::Effect, fields)
    }
    /// Local episode of an effect (§7.2, §25.1): a sub-slot written after its
    /// address. The exposure drawn once keeps its identity.
    pub fn episode(effect: &Self, slot: &str) -> Result<Self, HistoryError> {
        if effect.kind != AddressKind::Effect {
            return Err(invalid("address.effect_required"));
        }
        let mut fields = effect.fields.clone();
        fields.push(slot.into());
        Self::from_fields(AddressKind::Episode, fields)
    }
    /// Parent event of an impact address.
    pub fn parent_event(&self) -> Option<Self> {
        if self.kind != AddressKind::Impact {
            return None;
        }
        let fields = self.fields[..self.fields.len() - 1].to_vec();
        Some(Self {
            kind: AddressKind::Event,
            fields,
        })
    }
    pub fn site(world: &str, site_id: &str) -> Result<Self, HistoryError> {
        Self::from_fields(AddressKind::Site, vec![world.into(), site_id.into()])
    }
    pub fn activity(world: &str, activity_id: &str) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Activity,
            vec![world.into(), activity_id.into()],
        )
    }
    pub fn asset(owner: &Owner, kind: &str, block: i32, slot: &str) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Asset,
            owner,
            ["asset".into(), kind.into(), block.to_string(), slot.into()],
        )
    }
    pub fn group(
        owner: &Owner,
        archetype: &str,
        block: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Group,
            owner,
            [
                "group_slot".into(),
                archetype.into(),
                block.to_string(),
                ordinal.to_string(),
            ],
        )
    }
    pub fn household(domain: &DomainKey, origin: &str, year: i32) -> Result<Self, HistoryError> {
        Self::in_domain(
            AddressKind::Household,
            domain,
            ["household".into(), origin.into(), year.to_string()],
        )
    }
    pub fn transfer(
        owner: &Owner,
        origin: &str,
        destination: &str,
        year: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Transfer,
            owner,
            [
                "transfer".into(),
                origin.into(),
                destination.into(),
                year.to_string(),
                ordinal.to_string(),
            ],
        )
    }
    pub fn plan(domain: &DomainKey) -> Result<Self, HistoryError> {
        Self::in_domain(AddressKind::Plan, domain, [])
    }
    pub fn domain(domain: &DomainKey) -> Result<Self, HistoryError> {
        Self::in_domain(AddressKind::Domain, domain, [])
    }
    /// Relation between two addressable `ObjectRef`s. The common prefix of the
    /// endpoints is written once; each segment carries its number of fields, and
    /// the endpoint types stay explicit.
    pub fn relation(
        owner: &Owner,
        predicate: Predicate,
        subject: &ObjectRef,
        object: &ObjectRef,
        start: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        let head = vec![
            AddressKind::Relation.prefix().to_string(),
            predicate_name(predicate)?,
        ];
        Self::link_address(
            AddressKind::Relation,
            owner,
            head,
            subject,
            object,
            start,
            ordinal,
        )
    }
    /// Legal standing (§27.3): same shape as a relation, without a predicate.
    pub fn standing(
        owner: &Owner,
        subject: &ObjectRef,
        jurisdiction: &ObjectRef,
        start: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        let head = vec![AddressKind::Standing.prefix().to_string()];
        Self::link_address(
            AddressKind::Standing,
            owner,
            head,
            subject,
            jurisdiction,
            start,
            ordinal,
        )
    }
    fn link_address(
        kind: AddressKind,
        owner: &Owner,
        head: Vec<String>,
        subject: &ObjectRef,
        object: &ObjectRef,
        start: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        let (subject, object) = (subject.address()?, object.address()?);
        if subject.fields[0] != owner.world() || object.fields[0] != owner.world() {
            return Err(invalid("id.cross_canon_link"));
        }
        let common = subject
            .fields
            .iter()
            .zip(&object.fields)
            .take_while(|(a, b)| a == b)
            .count();
        let mut fields = owner.flat_fields();
        fields.extend(head);
        fields.extend([
            subject.kind.prefix().into(),
            object.kind.prefix().into(),
            (common - 1).to_string(),
        ]);
        fields.extend_from_slice(&subject.fields[1..common]);
        fields.push((subject.fields.len() - common).to_string());
        fields.extend_from_slice(&subject.fields[common..]);
        fields.push((object.fields.len() - common).to_string());
        fields.extend_from_slice(&object.fields[common..]);
        fields.extend([start.to_string(), ordinal.to_string()]);
        Self::from_fields(kind, fields)
    }
    /// Owner of the address: its domain for a person, a household, a plan or a
    /// domain; none for a site or an activity, which belong to the world.
    pub fn owner(&self) -> Option<Owner> {
        match self.kind {
            AddressKind::Person
            | AddressKind::Household
            | AddressKind::Plan
            | AddressKind::Domain => {
                let key = DomainKey {
                    world: self.fields[0].clone(),
                    region: self.fields[1].clone(),
                    society: self.fields[2].clone(),
                    ordinal: self.fields[3].parse().ok()?,
                };
                Some(Owner::domain(&key))
            }
            AddressKind::Site | AddressKind::Activity => None,
            _ => Owner::from_flat_fields(&self.fields),
        }
    }
    /// Fields that follow the owner.
    pub fn owned_tail(&self) -> &[String] {
        let start = match self.kind {
            AddressKind::Person
            | AddressKind::Household
            | AddressKind::Plan
            | AddressKind::Domain => DOMAIN_SLOTS.len(),
            AddressKind::Site | AddressKind::Activity => 1,
            _ => owner_slots(&self.fields).map_or(0, <[Slot]>::len),
        };
        &self.fields[start..]
    }
    /// Reading of a relation or a legal standing.
    pub fn link(&self) -> Option<LinkParts> {
        matches!(self.kind, AddressKind::Relation | AddressKind::Standing)
            .then(|| parse_link(self.kind, &self.fields).ok())
            .flatten()
    }
    fn encoded_len(&self) -> usize {
        let n = 5 + self.fields.iter().map(|s| 4 + s.len()).sum::<usize>();
        self.kind.prefix().len() + 1 + (n * 8).div_ceil(6)
    }
    pub fn encode(&self) -> String {
        let mut bytes = b"chr2\0".to_vec();
        for field in &self.fields {
            bytes.extend_from_slice(&(field.len() as u32).to_le_bytes());
            bytes.extend_from_slice(field.as_bytes());
        }
        format!("{}.{}", self.kind.prefix(), base64_encode(&bytes))
    }
    pub fn decode(id: &str) -> Result<Self, HistoryError> {
        if id.len() > MAX_ID_CHARS {
            return Err(invalid("id.too_long"));
        }
        let (prefix, payload) = id
            .split_once('.')
            .ok_or_else(|| invalid("id.prefix_missing"))?;
        let kind = AddressKind::from_prefix(prefix).ok_or_else(|| invalid("id.unknown_type"))?;
        let bytes = base64_decode(payload)?;
        if !bytes.starts_with(b"chr2\0") {
            return Err(invalid("id.signature_missing"));
        }
        let mut fields = Vec::new();
        let mut pos = 5;
        while pos < bytes.len() {
            let length = bytes
                .get(pos..pos + 4)
                .ok_or_else(|| invalid("id.length_truncated"))?;
            let length = u32::from_le_bytes(length.try_into().expect("four bytes")) as usize;
            pos += 4;
            let end = pos
                .checked_add(length)
                .ok_or_else(|| invalid("id.length_excessive"))?;
            let field = std::str::from_utf8(
                bytes
                    .get(pos..end)
                    .ok_or_else(|| invalid("id.field_truncated"))?,
            )
            .map_err(|_| invalid("id.invalid_utf8"))?;
            fields.push(field.to_owned());
            pos = end;
        }
        let address = Self::from_fields(kind, fields)?;
        if address.encode() != id {
            return Err(invalid("id.base64url_not_canonical"));
        }
        Ok(address)
    }
}
/// Owner, literal, closed predicate if any, endpoint types, then three counted
/// segments: common prefix, rest of the subject, rest of the object.
fn parse_link(kind: AddressKind, fields: &[String]) -> Result<LinkParts, HistoryError> {
    let owner = owner_slots(fields)?;
    let with_predicate = kind == AddressKind::Relation;
    let head = owner.len() + if with_predicate { 4 } else { 3 };
    let header = fields
        .get(..head)
        .ok_or_else(|| invalid("id.relation_truncated"))?;
    check_slots(&header[..owner.len()], owner)?;
    check_slot(&header[owner.len()], Literal(kind.prefix()))?;
    let predicate = if with_predicate {
        let name = header[owner.len() + 1].as_str();
        Some(
            serde_json::from_value::<Predicate>(name.into())
                .map_err(|_| invalid("id.unknown_predicate"))?,
        )
    } else {
        None
    };
    let subject_kind = AddressKind::from_endpoint_prefix(&header[head - 2])?;
    let object_kind = AddressKind::from_endpoint_prefix(&header[head - 1])?;
    let mut pos = head;
    let mut segment = || -> Result<&[String], HistoryError> {
        let count = fields
            .get(pos)
            .ok_or_else(|| invalid("id.relation_truncated"))?;
        let n: usize = count
            .parse()
            .map_err(|_| invalid("id.bad_segment_length"))?;
        // Guard before any addition: `n` comes from an untrusted ID.
        if n.to_string() != *count || n > fields.len() {
            return Err(invalid("id.segment_not_canonical"));
        }
        let end = pos + 1 + n;
        if end > fields.len() {
            return Err(invalid("id.bad_relation_segments"));
        }
        let values = &fields[pos + 1..end];
        pos = end;
        Ok(values)
    };
    let common = segment()?;
    let subject_rest = segment()?;
    let object_rest = segment()?;
    if pos + 2 != fields.len() {
        return Err(invalid("id.bad_relation_segments"));
    }
    if !subject_rest.is_empty() && subject_rest.first() == object_rest.first() {
        return Err(invalid("id.prefix_not_maximal"));
    }
    let endpoint = |rest: &[String]| [std::slice::from_ref(&fields[0]), common, rest].concat();
    let (subject, object) = (endpoint(subject_rest), endpoint(object_rest));
    check_structure(subject_kind, &subject)?;
    check_structure(object_kind, &object)?;
    check_slot(&fields[pos], Year)?;
    check_slot(&fields[pos + 1], Ordinal)?;
    Ok(LinkParts {
        owner: Owner::from_flat_fields(fields).ok_or_else(|| invalid("id.unreadable_owner"))?,
        predicate,
        subject: SemanticAddress {
            kind: subject_kind,
            fields: subject,
        },
        object: SemanticAddress {
            kind: object_kind,
            fields: object,
        },
        start: fields[pos]
            .parse()
            .map_err(|_| invalid("id.bad_link_year"))?,
        ordinal: fields[pos + 1]
            .parse()
            .map_err(|_| invalid("id.ordinal_out_of_range"))?,
    })
}
/// Fields of the parent contract, then typed target.
fn validate_effect_fields(fields: &[String]) -> Result<(), HistoryError> {
    let impact_end = owner_slots(fields)?.len() + 4;
    let impact = fields
        .get(..impact_end)
        .ok_or_else(|| invalid("id.effect_truncated"))?;
    check_structure(AddressKind::Impact, impact)?;
    let [target_kind, target @ ..] = &fields[impact_end..] else {
        return Err(invalid("id.effect_truncated"));
    };
    let kind = AddressKind::from_endpoint_prefix(target_kind)?;
    check_structure(kind, &[std::slice::from_ref(&fields[0]), target].concat())
}
/// Shape of an address, without searching for a nested ID: it is also the
/// criterion for recognizing a payload, which must not recall itself.
fn check_structure(kind: AddressKind, fields: &[String]) -> Result<(), HistoryError> {
    if fields.is_empty()
        || fields
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|b| (0x21..=0x7e).contains(&b)))
    {
        return Err(invalid("id.fields_not_normalized"));
    }
    match kind {
        AddressKind::Relation | AddressKind::Standing => parse_link(kind, fields).map(|_| ()),
        AddressKind::Effect => validate_effect_fields(fields),
        AddressKind::Episode => validate_effect_fields(&fields[..fields.len() - 1]),
        _ => check_slots(fields, &fixed_slots(kind, fields)?),
    }
}
/// True if a substring of the field decodes to a complete address. A payload may
/// be followed by characters of the base64url alphabet: every cut at a field
/// end is tried, and only a canonical cut counts.
fn embeds_address(field: &str) -> bool {
    field.match_indices(ENCODED_SIGNATURE).any(|(start, _)| {
        let rest = &field[start..];
        let end = rest
            .bytes()
            .position(|b| !BASE64.contains(&b))
            .unwrap_or(rest.len());
        starts_with_address(&rest[..end])
    })
}
fn starts_with_address(run: &str) -> bool {
    let Ok((bytes, _)) = base64_unpack(run) else {
        return false;
    };
    if !bytes.starts_with(b"chr2\0") {
        return false;
    }
    let mut fields = Vec::new();
    let mut pos = 5;
    while let Some(length) = bytes.get(pos..pos + 4) {
        let length = u32::from_le_bytes(length.try_into().expect("four bytes")) as usize;
        let field = (pos + 4)
            .checked_add(length)
            .and_then(|end| bytes.get(pos + 4..end))
            .and_then(|field| std::str::from_utf8(field).ok());
        let Some(field) = field else {
            return false;
        };
        fields.push(field.to_owned());
        pos += 4 + length;
        if base64_decode(&run[..(pos * 8).div_ceil(6)]).is_ok()
            && AddressKind::ALL
                .into_iter()
                .any(|kind| check_structure(kind, &fields).is_ok())
        {
            return true;
        }
    }
    false
}
const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
fn base64_encode(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for b in bytes {
        buffer = (buffer << 8) | u32::from(*b);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            result.push(BASE64[((buffer >> bits) & 63) as usize] as char);
        }
    }
    if bits > 0 {
        result.push(BASE64[((buffer << (6 - bits)) & 63) as usize] as char);
    }
    result
}
fn base64_decode(s: &str) -> Result<Vec<u8>, HistoryError> {
    if s.len() % 4 == 1 {
        return Err(invalid("id.bad_base64url_length"));
    }
    let (out, padding) = base64_unpack(s)?;
    if padding != 0 {
        return Err(invalid("id.nonzero_padding"));
    }
    Ok(out)
}
/// Whole bytes of the string and the value of its trailing bits.
fn base64_unpack(s: &str) -> Result<(Vec<u8>, u32), HistoryError> {
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for b in s.bytes() {
        let v = BASE64
            .iter()
            .position(|c| *c == b)
            .ok_or_else(|| invalid("id.bad_base64url_alphabet"))?;
        buffer = (buffer << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Ok((out, buffer & ((1 << bits) - 1)))
}

/// Single entry point of v2 random decisions. Sub-counters and rejection
/// attempts are explicit fields, never a consumed global state.
pub mod random_semantics_v2 {
    use super::{ErrorCode, HistoryError};
    /// The seed leaves the world (canon) field of a domain address out: a manifest
    /// change that does not touch the domain itself (name pools, pinned runtime) must
    /// not redraw its demography. An ID that is not a domain address is used as is.
    pub fn domain_seed(world_seed: u64, domain_id: &str, provider_algorithm: &str) -> u64 {
        let local = super::DomainKey::from_id(domain_id)
            .map(|k| format!("{}\u{0}{}\u{0}{}", k.region(), k.society(), k.ordinal()));
        let local = local.as_deref().unwrap_or(domain_id);
        crate::keys::key(
            world_seed,
            &["population/domain-seed/v3", local, provider_algorithm],
        )
    }
    pub fn draw(seed: u64, parts: &[&str], subcounter: u64, rejection: u64) -> u64 {
        let sub = subcounter.to_string();
        let retry = rejection.to_string();
        // ASCII rules out the 0xff separator of the v1 mixer. The number of
        // fields prevents any confusion with suffixed sub-counters.
        let count = parts.len().to_string();
        let encoded: Vec<_> = parts.iter().map(|s| base64_part(s.as_bytes())).collect();
        let mut key = vec!["random_semantics_v2", count.as_str()];
        key.extend(encoded.iter().map(String::as_str));
        key.extend([sub.as_str(), retry.as_str()]);
        crate::keys::key(seed, &key)
    }
    fn base64_part(bytes: &[u8]) -> String {
        super::base64_encode(bytes)
    }
    /// Exact uniform on [0, upper), including bounds that are not powers of 2.
    pub fn uniform_below(
        seed: u64,
        parts: &[&str],
        subcounter: u64,
        upper: u64,
    ) -> Result<u64, HistoryError> {
        if upper == 0 {
            return Err(HistoryError::new(
                ErrorCode::InvalidCanonicalValue,
                "id.uniform_bound_zero",
            ));
        }
        let threshold = upper.wrapping_neg() % upper;
        for retry in 0..=u64::MAX {
            let value = draw(seed, parts, subcounter, retry);
            if value >= threshold {
                return Ok(value % upper);
            }
        }
        Err(HistoryError::new(
            ErrorCode::BudgetExceeded,
            "id.rejection_counter_exhausted",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::model::ObjectRef;
    fn domain() -> DomainKey {
        DomainKey::new("w", "region", "society", 0).unwrap()
    }
    fn fields(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).into()).collect()
    }
    fn office() -> SemanticAddress {
        SemanticAddress::entity(&Owner::cell("w", "r"), "office", 0, "mayor").unwrap()
    }
    fn person(local: &str) -> ObjectRef {
        let a = SemanticAddress::person(&domain(), "x", local).unwrap();
        ObjectRef::PersonSlot(a.encode())
    }
    fn forge(prefix: &str, values: &[&str]) -> String {
        let mut bytes = b"chr2\0".to_vec();
        for v in values {
            bytes.extend_from_slice(&(v.len() as u32).to_le_bytes());
            bytes.extend_from_slice(v.as_bytes());
        }
        format!("{prefix}.{}", base64_encode(&bytes))
    }
    #[test]
    fn binary_address_has_reference_encoding() {
        let a = SemanticAddress::domain(&domain()).unwrap();
        assert_eq!(
            a.encode(),
            "domain.Y2hyMgABAAAAdwYAAAByZWdpb24HAAAAc29jaWV0eQEAAAAw"
        );
        assert_eq!(SemanticAddress::decode(&a.encode()).unwrap(), a);
    }
    #[test]
    fn address_prefix_uses_a_dot_and_refuses_the_former_colon() {
        let id = SemanticAddress::domain(&domain()).unwrap().encode();
        assert!(id.starts_with("domain.Y2hyMg"));
        assert!(SemanticAddress::decode(&id.replacen('.', ":", 1)).is_err());
    }
    #[test]
    fn decimal_address_fields_are_normalized() {
        for value in ["01", "+1", "-0", "1.0", "x"] {
            assert!(SemanticAddress::from_fields(
                AddressKind::Domain,
                fields(&["w", "r", "s", value])
            )
            .is_err());
        }
        assert!(SemanticAddress::decode("domain.Y2hyMgABAAAAdwEAAAByAQAAAHMBAAAAMB").is_err());
    }
    #[test]
    fn every_object_has_a_single_address_form() {
        // Former flattened or joined forms, all rejected.
        for (kind, old) in [
            (
                AddressKind::Person,
                vec!["w", "region/society/0", "a", "17"],
            ),
            (AddressKind::Plan, vec!["w", "region/society/0"]),
            (AddressKind::Tenure, vec!["w", "office", "0"]),
            (
                AddressKind::Household,
                vec!["w", "d", "household", "o", "1354"],
            ),
            (AddressKind::Event, vec!["w", "scope", "k", "1350", "0"]),
            (
                AddressKind::Group,
                vec!["w", "s", "group_slot", "a", "1350", "0"],
            ),
            (
                AddressKind::Transfer,
                vec!["w", "s", "transfer", "a", "b", "1354", "0"],
            ),
            (AddressKind::Entity, vec!["w", "s", "k", "1350", "x"]),
            (
                AddressKind::Asset,
                vec!["w", "cell", "r", "mill", "1350", "x"],
            ),
            (AddressKind::Site, vec!["w", "cell", "site/a"]),
        ] {
            assert!(
                SemanticAddress::from_fields(kind, fields(&old)).is_err(),
                "{kind:?}"
            );
        }
    }
    #[test]
    fn every_owner_variant_is_flat_and_read_back_from_its_address() {
        let d = domain();
        for (owner, flat) in [
            (Owner::whole_world("w"), vec!["w", "world"]),
            (Owner::cell("w", "r"), vec!["w", "cell", "r"]),
            (Owner::site("w", "site/a"), vec!["w", "site", "site/a"]),
            (
                Owner::domain(&d),
                vec!["w", "domain", "region", "society", "0"],
            ),
        ] {
            for address in [
                SemanticAddress::event(&owner, "plague", 1350, 2).unwrap(),
                SemanticAddress::group(&owner, "farm", 1350, 2).unwrap(),
                SemanticAddress::transfer(&owner, "a", "b", 1354, 2).unwrap(),
                SemanticAddress::entity(&owner, "office", 1350, "mayor").unwrap(),
                SemanticAddress::asset(&owner, "mill", 1350, "x").unwrap(),
            ] {
                assert_eq!(&address.fields()[..flat.len()], flat);
                assert_eq!(address.owner().as_ref(), Some(&owner));
                assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
            }
        }
        for address in [
            SemanticAddress::person(&d, "algo", "17").unwrap(),
            SemanticAddress::household(&d, "singleton/17", 1354).unwrap(),
            SemanticAddress::plan(&d).unwrap(),
        ] {
            assert_eq!(&address.fields()[..4], ["w", "region", "society", "0"]);
            assert_eq!(address.owner(), Some(Owner::domain(&d)));
            assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
        }
    }
    #[test]
    fn owner_scopes_never_share_an_address() {
        let label = "region/society/0";
        let owners = [
            Owner::cell("w", label),
            Owner::site("w", label),
            Owner::domain(&domain()),
            Owner::whole_world("w"),
        ];
        let ids: std::collections::BTreeSet<_> = owners
            .iter()
            .map(|o| SemanticAddress::event(o, "k", 0, 0).unwrap().encode())
            .collect();
        assert_eq!(ids.len(), owners.len());
    }
    #[test]
    fn site_and_activity_addresses_belong_to_the_world() {
        for address in [
            SemanticAddress::site("w", "site/a").unwrap(),
            SemanticAddress::activity("w", "agriculture").unwrap(),
        ] {
            assert_eq!(address.fields()[0], "w");
            assert_eq!(address.owner(), None);
            assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
        }
        assert_ne!(
            SemanticAddress::site("w", "x").unwrap().encode(),
            SemanticAddress::activity("w", "x").unwrap().encode()
        );
    }
    #[test]
    fn domain_key_reads_back_only_from_a_domain_id() {
        let d = domain();
        let id = SemanticAddress::domain(&d).unwrap().encode();
        assert_eq!(DomainKey::from_id(&id).unwrap(), d);
        let plan = SemanticAddress::plan(&d).unwrap().encode();
        assert!(DomainKey::from_id(&plan).is_err());
    }
    #[test]
    fn tenure_requires_an_office_entity() {
        let tenure = SemanticAddress::tenure(&office(), 3).unwrap();
        assert_eq!(SemanticAddress::decode(&tenure.encode()).unwrap(), tenure);
        let other = SemanticAddress::entity(&Owner::cell("w", "r"), "guild", 0, "x").unwrap();
        assert!(SemanticAddress::tenure(&other, 0).is_err());
        let person = SemanticAddress::person(&domain(), "a", "1").unwrap();
        assert!(SemanticAddress::tenure(&person, 0).is_err());
    }
    #[test]
    fn literal_fields_are_checked_when_decoding() {
        let good = forge("group", &["w", "cell", "r", "group_slot", "a", "0", "0"]);
        assert!(SemanticAddress::decode(&good).is_ok());
        for (prefix, forged) in [
            ("group", vec!["w", "cell", "r", "x", "a", "0", "0"]),
            ("household", vec!["w", "r", "s", "0", "x", "o", "1354"]),
            (
                "transfer",
                vec!["w", "cell", "r", "x", "a", "b", "1354", "0"],
            ),
            ("asset", vec!["w", "cell", "r", "x", "mill", "0", "s"]),
        ] {
            assert!(SemanticAddress::decode(&forge(prefix, &forged)).is_err());
        }
        let owner = Owner::cell("w", "r");
        let relation = SemanticAddress::relation(
            &owner,
            Predicate::Trusts,
            &person("1"),
            &person("2"),
            1300,
            0,
        )
        .unwrap();
        let mut forged = relation.fields().to_vec();
        forged[3] = "x".into();
        let forged: Vec<_> = forged.iter().map(String::as_str).collect();
        assert!(SemanticAddress::decode(&forge("relation", &forged)).is_err());
    }
    #[test]
    fn relation_refuses_an_unknown_predicate_when_decoding() {
        let owner = Owner::cell("w", "r");
        let r = SemanticAddress::relation(
            &owner,
            Predicate::ParentOf,
            &person("1"),
            &person("2"),
            -5,
            0,
        )
        .unwrap();
        assert_eq!(SemanticAddress::decode(&r.encode()).unwrap(), r);
        assert_eq!(r.fields()[4], "parent_of");
        let mut fields = r.fields().to_vec();
        fields[4] = "not_a_predicate".into();
        assert!(SemanticAddress::from_fields(AddressKind::Relation, fields).is_err());
    }
    #[test]
    fn relation_segments_are_exact_and_maximal() {
        let owner = Owner::cell("w", "r");
        let (a, b) = (person("1"), person("2"));
        let self_loop = SemanticAddress::relation(&owner, Predicate::Trusts, &a, &a, 0, 0).unwrap();
        assert_eq!(
            SemanticAddress::decode(&self_loop.encode()).unwrap(),
            self_loop
        );
        let other = Owner::cell("v", "r");
        assert!(SemanticAddress::relation(&other, Predicate::Trusts, &a, &b, 0, 0).is_err());
        let relation = SemanticAddress::relation(&owner, Predicate::Trusts, &a, &b, 0, 0).unwrap();
        let nested = ObjectRef::Entity(relation.encode());
        assert!(SemanticAddress::relation(&owner, Predicate::Trusts, &nested, &b, 0, 0).is_err());
        // Same relation written by hand: maximal prefix accepted, shortened
        // prefix rejected for the same content.
        let head = ["w", "cell", "r", "relation", "trusts", "person", "person"];
        let maximal = [
            &head[..],
            &[
                "4", "region", "society", "0", "x", "1", "1", "1", "2", "0", "0",
            ],
        ]
        .concat();
        assert_eq!(
            SemanticAddress::from_fields(AddressKind::Relation, fields(&maximal)).unwrap(),
            relation
        );
        let unshortened = [
            &head[..],
            &[
                "3", "region", "society", "0", "2", "x", "1", "2", "x", "2", "0", "0",
            ],
        ]
        .concat();
        assert!(SemanticAddress::from_fields(AddressKind::Relation, fields(&unshortened)).is_err());
    }
    #[test]
    fn relation_endpoints_are_object_refs_only() {
        // An event is not an endpoint, even by hand.
        let forged = forge(
            "relation",
            &[
                "w", "cell", "r", "relation", "trusts", "event", "person", "0", "6", "cell", "r",
                "k", "0", "0", "x", "5", "region", "society", "0", "x", "1", "0", "0",
            ],
        );
        assert!(SemanticAddress::decode(&forged).is_err());
    }
    #[test]
    fn impact_and_effect_are_flat_children_of_their_event() {
        let owner = Owner::cell("w", "r");
        let event = SemanticAddress::event(&owner, "war", 1350, 0).unwrap();
        let impact = SemanticAddress::impact(&event, "impact-0").unwrap();
        assert_eq!(&impact.fields()[..event.fields().len()], event.fields());
        assert_eq!(impact.parent_event().as_ref(), Some(&event));
        assert_eq!(impact.owner().as_ref(), Some(&owner));
        assert_eq!(SemanticAddress::decode(&impact.encode()).unwrap(), impact);
        let site = SemanticAddress::site("w", "site/a").unwrap();
        let effect = SemanticAddress::effect(&impact, &site).unwrap();
        assert_eq!(&effect.fields()[..impact.fields().len()], impact.fields());
        assert_eq!(effect.owner().as_ref(), Some(&owner));
        assert_eq!(SemanticAddress::decode(&effect.encode()).unwrap(), effect);
        let other = SemanticAddress::site("w", "site/b").unwrap();
        assert_ne!(effect, SemanticAddress::effect(&impact, &other).unwrap());
        let group = SemanticAddress::group(&owner, "farm", 1350, 1).unwrap();
        let flat = SemanticAddress::effect(&impact, &group).unwrap();
        assert!(!flat.fields().contains(&group.encode()));
        assert_eq!(SemanticAddress::decode(&flat.encode()).unwrap(), flat);
        // Another canon, wrong address type: rejected.
        let foreign = SemanticAddress::site("v", "site/a").unwrap();
        assert!(SemanticAddress::effect(&impact, &foreign).is_err());
        assert!(SemanticAddress::effect(&event, &site).is_err());
        assert!(SemanticAddress::impact(&group, "impact-0").is_err());
        assert!(SemanticAddress::effect(&impact, &event).is_err());
    }
    #[test]
    fn decade_block_uses_euclidean_division_before_year_zero() {
        for (year, block) in [
            (0, 0),
            (9, 0),
            (10, 10),
            (1354, 1350),
            (-1, -10),
            (-10, -10),
            (-11, -20),
            (-100_000, -100_000),
            (100_000, 100_000),
        ] {
            assert_eq!(decade_block(year), block, "{year}");
        }
    }
    #[test]
    fn origin_block_must_be_the_aligned_start_year() {
        let owner = Owner::cell("w", "r");
        assert!(SemanticAddress::event(&owner, "k", -10, 0).is_ok());
        for block in [1351, 1, -1, -15] {
            assert!(SemanticAddress::event(&owner, "k", block, 0).is_err());
            assert!(SemanticAddress::group(&owner, "a", block, 0).is_err());
            assert!(SemanticAddress::entity(&owner, "k", block, "s").is_err());
            assert!(SemanticAddress::asset(&owner, "k", block, "s").is_err());
        }
    }
    #[test]
    fn years_and_blocks_are_bounded() {
        let owner = Owner::cell("w", "r");
        assert!(SemanticAddress::event(&owner, "k", 100_000, 0).is_ok());
        assert!(SemanticAddress::event(&owner, "k", -100_000, 0).is_ok());
        assert!(SemanticAddress::event(&owner, "k", 100_010, 0).is_err());
        assert!(SemanticAddress::event(&owner, "k", -100_010, 0).is_err());
        assert!(SemanticAddress::household(&domain(), "o", 100_000).is_ok());
        assert!(SemanticAddress::household(&domain(), "o", 100_001).is_err());
        assert!(SemanticAddress::transfer(&owner, "a", "b", -100_001, 0).is_err());
        let a = person("1");
        assert!(SemanticAddress::relation(&owner, Predicate::Helps, &a, &a, 100_001, 0).is_err());
    }
    #[test]
    fn domain_seed_is_pinned_by_a_literal_value() {
        // Oracle: independent Python port of keys.rs, seed 42.
        assert_eq!(
            random_semantics_v2::domain_seed(42, "d", "a"),
            4511880416797658050
        );
    }
    #[test]
    fn domain_seed_ignores_the_world_of_the_domain_address() {
        use random_semantics_v2::domain_seed;
        let id = |world: &str| {
            let key = DomainKey::new(world, "region/demo", "society/abc", 0).unwrap();
            SemanticAddress::domain(&key).unwrap().encode()
        };
        assert_eq!(
            domain_seed(11, &id("canon-a"), "p"),
            domain_seed(11, &id("canon-b"), "p")
        );
        assert_ne!(
            domain_seed(11, &id("canon-a"), "p"),
            domain_seed(12, &id("canon-a"), "p")
        );
    }
    #[test]
    fn draws_are_pinned_by_literal_values() {
        use random_semantics_v2::*;
        assert_eq!(draw(42, &["a"], 0, 0), 11994162603066947378);
        assert_eq!(draw(42, &["a"], 1, 0), 5619045440094019664);
        assert_eq!(draw(42, &["a"], 0, 1), 10113000962104654725);
        assert_eq!(draw(42, &["a", "b"], 0, 0), 11532361124723730494);
        assert_eq!(draw(43, &["a"], 0, 0), 76182163578657930);
        assert_eq!(uniform_below(42, &["a"], 0, 10_000).unwrap(), 7378);
    }
    #[test]
    fn exact_uniform_rejects_the_biased_prefix_and_uses_explicit_retry() {
        use random_semantics_v2::*;
        let upper = (1u64 << 63) + 1;
        let threshold = upper.wrapping_neg() % upper;
        let seed = (0..100)
            .find(|seed| {
                draw(*seed, &["rejection-reference"], 7, 0) < threshold
                    && draw(*seed, &["rejection-reference"], 7, 1) >= threshold
            })
            .unwrap();
        assert_eq!(
            uniform_below(seed, &["rejection-reference"], 7, upper).unwrap(),
            draw(seed, &["rejection-reference"], 7, 1) % upper
        );
        assert_eq!(
            uniform_below(seed, &["rejection-reference"], 7, 1).unwrap(),
            0
        );
        assert!(uniform_below(seed, &[], 0, 0).is_err());
    }
    #[test]
    fn extreme_years_in_malformed_ids_are_refused_without_panicking() {
        for year in [
            "-9223372036854775808",
            "9223372036854775807",
            "-9223372036854775809",
            "100001",
        ] {
            let household = ["w", "r", "s", "0", "household", "o", year];
            assert!(SemanticAddress::decode(&forge("household", &household)).is_err());
        }
    }
    #[test]
    fn extreme_relation_counts_in_malformed_ids_are_refused_without_panicking() {
        let head = ["w", "cell", "r", "relation", "trusts", "person", "person"];
        for count in [
            "18446744073709551615",
            "18446744073709551614",
            "9223372036854775807",
            "30",
        ] {
            let tails: [&[&str]; 3] = [
                &[count, "1", "x", "1", "y", "0", "0"],
                &["0", count, "x", "1", "y", "0", "0"],
                &["0", "1", "x", count, "y", "0", "0"],
            ];
            for tail in tails {
                let fields = [&head[..], tail].concat();
                assert!(SemanticAddress::decode(&forge("relation", &fields)).is_err());
            }
        }
    }
    #[test]
    fn readable_fields_may_contain_the_signature_when_no_address_decodes() {
        let owner = Owner::cell("w", "r");
        for field in [
            "Y2hyMg",
            "site/Y2hyMg/1",
            "xY2hyMgz",
            "Y2hyMgA",
            "Y2hyMgAB",
            "Y2hyMgABAAAA",
            "Y2hyMgAyyyyyyyyyyyyyyyyyyyy",
        ] {
            let address = SemanticAddress::event(&owner, field, 0, 0).unwrap();
            assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
        }
    }
    #[test]
    fn encoded_ids_are_refused_in_every_field_position() {
        let parent = office();
        let id = parent.encode();
        let payload = id.split_once('.').unwrap().1;
        let owner = Owner::cell("w", "r");
        for field in [
            id.clone(),
            payload.to_string(),
            format!("x-{id}-y"),
            format!("x-{payload}-y"),
            format!("{id}/suffixe"),
            // Double nesting: the inner signature is re-encoded.
            SemanticAddress::event(&Owner::cell("v", "r"), "k", 0, 0)
                .map(|e| forge("event", &["w", "cell", "r", &e.encode(), "0", "0"]))
                .unwrap(),
        ] {
            assert!(
                SemanticAddress::event(&owner, &field, 0, 0).is_err(),
                "{field}"
            );
            assert!(SemanticAddress::entity(&owner, "k", 0, &field).is_err());
            assert!(SemanticAddress::person(&domain(), "a", &field).is_err());
            assert!(DomainKey::new("w", &field, "s", 0).is_err());
            assert!(SemanticAddress::event(&Owner::cell("w", &field), "k", 0, 0).is_err());
        }
    }
}
