//! `project_view`, the pure read side of the API (§14.1, §14.3): materialized packets
//! in, a bounded `HistoryView` out. Nothing here draws, and nothing is invented to fill
//! a missing packet: an unmaterialized domain is an error, not a guess (§14.1).
//!
//! Identity rule: every exposed ID is `entity.id` or a person address. The packet-local
//! group index and the person slot never leave this module.
use super::atlas::{ContactEnd, HistoryAtlas};
use super::canonical::canonical_bytes;
use super::display_names::{
    name_provenance, place_labels, polity_name, render_tree, same_text, PlaceLabel,
    DISPLAY_NAME_RULE,
};
use super::general::{marker_dates, GeneralCanon};
use super::ids::{AddressKind, DomainKey, Owner, OwnerScope, SemanticAddress};
use super::influence::influence_at;
use super::model::{
    conduct_class, ConductClass, ConductThresholds, ErrorCode, HistoryError, HistoryEvent,
    LegalStanding, LegalStatus, ObjectRef, Predicate, Provenance, SocialRelation, TimeSpan,
};
use super::offices::{resolve_office, OfficeDefinition, Tenure, TenureMode};
use super::population::{SlotSource, VerifiedPopulation};
use super::region::RegionPlan;
use super::social::{
    family_of, group_state, members_at, membership_relation, person_record, state_span,
    GroupRecord, Membership, SocialPacket,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const VIEW_SCHEMA: &str = "chronos.history-view.v2";
/// Drawing caps of the web graph (§15.3), echoed in every view.
pub const MAX_NODES: usize = 300;
pub const MAX_EDGES: usize = 1000;
/// Members listed when a group is opened, so that a guild never floods the graph.
const MEMBER_CAP: usize = 16;
const EVENT_CAP: usize = 400;
const CAUSE_DEPTH: usize = 12;
/// Influence channels of §11, then the legality channel of §27.6.
pub const LEGALITY_CHANNEL: &str = "legality";

/// One materialized domain: the three artifacts `project_view` reads.
#[derive(Clone, Copy)]
pub struct Materialized<'a> {
    pub region: &'a RegionPlan,
    pub population: &'a VerifiedPopulation,
    pub packet: &'a SocialPacket,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct QueryScope {
    pub site_id: Option<String>,
    pub domain_id: Option<String>,
    /// Entities to open: persons, groups, offices. Their neighbourhood is returned.
    pub ids: Vec<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct QueryLimits {
    pub people: usize,
    pub relations: usize,
    pub groups: usize,
}
impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            people: 120,
            relations: 600,
            groups: 40,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Detail {
    Summary,
    #[default]
    Local,
    Extended,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ViewQuery {
    pub scope: QueryScope,
    pub at_year: Option<i32>,
    pub period: Option<TimeSpan>,
    /// Layers: `offices`, `organizations`, `people`, `events`, `measures`, `overlay`.
    /// Empty means all of them.
    pub include: Vec<String>,
    pub detail: Detail,
    pub limits: QueryLimits,
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HistoryView {
    pub schema: String,
    pub canon_id: String,
    /// Left empty by the core, which has no hash: the host fills it from
    /// `fingerprint_bytes` (§4.2).
    pub view_fingerprint: String,
    pub provenance: Value,
    /// Window of the facts returned; `timeline` is the whole span of the canon.
    pub period: Value,
    pub timeline: Value,
    pub scope: Value,
    pub at_year: i32,
    pub observed_start: i32,
    pub default_selection: Value,
    pub coverage: String,
    pub coverage_detail: Value,
    pub limits: Value,
    pub conduct_thresholds: Value,
    pub entities: Vec<Value>,
    pub offices: Vec<Value>,
    pub relations: Vec<Value>,
    pub legal_standings: Vec<Value>,
    pub events: Vec<Value>,
    pub tenures: Vec<Value>,
    pub measures: Vec<Value>,
    pub balances: Value,
    pub overlay: Value,
}
impl HistoryView {
    /// Canonical bytes of the view with an empty fingerprint: the host hashes them.
    pub fn fingerprint_bytes(&self) -> Result<Vec<u8>, HistoryError> {
        let mut blank = self.clone();
        blank.view_fingerprint = String::new();
        let value = serde_json::to_value(&blank)
            .map_err(|e| HistoryError::new(ErrorCode::InvalidCanonicalValue, e.to_string()))?;
        canonical_bytes(&value)
    }
}

fn fail(key: &str, owner: &str, year: Option<i32>) -> HistoryError {
    HistoryError {
        owner: Some(owner.into()),
        at_year: year,
        ..HistoryError::new(ErrorCode::InvalidRecordIdentity, key)
    }
}
fn fnv64(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}
fn tag_of(id: &str) -> String {
    format!("{:04x}", fnv64(id) & 0xffff)
}
fn tail(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}
/// Spans are half-open; an end at or after the timeline's end is "still open".
fn span_json(span: TimeSpan, horizon: i32) -> Value {
    json!([
        span.start_year,
        if span.end_year >= horizon {
            Value::Null
        } else {
            json!(span.end_year)
        }
    ])
}
fn provenance_json(p: &Provenance) -> Value {
    json!({"status": p.status, "source": p.source_ref, "rule_id": p.rule_id})
}
fn ref_id(r: &ObjectRef) -> &str {
    match r {
        ObjectRef::Entity(id)
        | ObjectRef::Office(id)
        | ObjectRef::Site(id)
        | ObjectRef::Asset(id)
        | ObjectRef::PersonSlot(id)
        | ObjectRef::Activity(id) => id,
    }
}
fn thresholds_json(t: &ConductThresholds) -> Value {
    json!([
        [
            "lawless",
            t.lawless_morality_max_bps,
            t.lawless_lawfulness_max_bps
        ],
        ["crooked", t.crooked_morality_max_bps],
        ["ordinary", t.ordinary_morality_below_bps],
        ["upright", t.upright_morality_below_bps],
    ])
}
fn measure(
    value: Option<u64>,
    unit: &str,
    mode: &str,
    coverage: &str,
    basis: &str,
    year: i32,
) -> Value {
    json!({"value": value, "unit": unit, "precision": mode, "coverage": coverage,
           "basis": basis, "at_year": year})
}

/// Domain that holds an ID, read from its address alone. `None` for an object owned by
/// a cell or the world, which needs no packet.
pub fn domain_of_id(canon: &GeneralCanon, id: &str) -> Option<String> {
    let address = SemanticAddress::decode(id).ok()?;
    let key_of = |fields: &[String]| {
        DomainKey::new(&fields[0], &fields[1], &fields[2], fields[3].parse().ok()?).ok()
    };
    match address.kind() {
        AddressKind::Person | AddressKind::Household | AddressKind::Plan | AddressKind::Domain => {
            let key = key_of(address.fields())?;
            SemanticAddress::domain(&key).ok().map(|a| a.encode())
        }
        AddressKind::Site | AddressKind::Activity => canon
            .domain_of_site(address.fields().get(1)?)
            .map(|d| d.domain_id.clone()),
        _ => match address.owner()?.scope() {
            OwnerScope::Site { site_id } => {
                canon.domain_of_site(site_id).map(|d| d.domain_id.clone())
            }
            OwnerScope::Domain { .. } => {
                let fields = address.fields();
                let key = key_of(&fields[..4]).or_else(|| {
                    DomainKey::new(
                        &fields[0],
                        match address.owner()?.scope() {
                            OwnerScope::Domain { region, .. } => region,
                            _ => return None,
                        },
                        match address.owner()?.scope() {
                            OwnerScope::Domain { society, .. } => society,
                            _ => return None,
                        },
                        match address.owner()?.scope() {
                            OwnerScope::Domain { ordinal, .. } => *ordinal,
                            _ => return None,
                        },
                    )
                    .ok()
                })?;
                SemanticAddress::domain(&key).ok().map(|a| a.encode())
            }
            _ => None,
        },
    }
}

// --- Request state ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Office(usize),
    Group(u32),
    Person(u32),
    Polity(usize),
}
struct Cx<'a> {
    canon: &'a GeneralCanon,
    m: Materialized<'a>,
    at_year: i32,
    period: TimeSpan,
    horizon: i32,
    group_by_id: BTreeMap<&'a str, u32>,
    slot_by_id: BTreeMap<&'a str, u32>,
    office_by_id: BTreeMap<&'a str, usize>,
    polity_by_id: BTreeMap<&'a str, usize>,
    memberships_by_slot: BTreeMap<u32, Vec<usize>>,
    unions_by_slot: BTreeMap<u32, Vec<usize>>,
    tenures_by_slot: BTreeMap<u32, Vec<usize>>,
    owner: Owner,
    /// Real names of sites, regions and polities.
    labels: BTreeMap<String, PlaceLabel>,
}
impl<'a> Cx<'a> {
    fn new(
        canon: &'a GeneralCanon,
        m: Materialized<'a>,
        at_year: i32,
        period: TimeSpan,
    ) -> Result<Self, HistoryError> {
        let packet = m.packet;
        let mut memberships_by_slot: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (i, membership) in packet.memberships.iter().enumerate() {
            memberships_by_slot
                .entry(membership.person_slot)
                .or_default()
                .push(i);
        }
        let mut unions_by_slot: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (i, union) in m.population.result().union_allocations.iter().enumerate() {
            for slot in &union.partner_slots {
                unions_by_slot.entry(*slot).or_default().push(i);
            }
        }
        let mut tenures_by_slot: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (i, tenure) in packet.tenures.iter().enumerate() {
            if let Some(slot) = tenure.person_slot {
                tenures_by_slot.entry(slot).or_default().push(i);
            }
        }
        let key = DomainKey::from_id(&packet.domain_id)?;
        Ok(Self {
            labels: place_labels(canon)?,
            canon,
            m,
            at_year,
            period,
            horizon: canon.timeline.end_year,
            group_by_id: packet
                .groups
                .iter()
                .enumerate()
                .map(|(i, g)| (g.entity.id.as_str(), i as u32))
                .collect(),
            slot_by_id: packet
                .people
                .iter()
                .enumerate()
                .map(|(i, p)| (p.person_id.as_str(), i as u32))
                .collect(),
            office_by_id: packet
                .offices
                .iter()
                .enumerate()
                .map(|(i, o)| (o.office_id.as_str(), i))
                .collect(),
            polity_by_id: canon
                .polities
                .iter()
                .enumerate()
                .map(|(i, p)| (p.entity.id.as_str(), i))
                .collect(),
            memberships_by_slot,
            unions_by_slot,
            tenures_by_slot,
            owner: Owner::domain(&key),
        })
    }
    fn group(&self, g: u32) -> &'a GroupRecord {
        &self.m.packet.groups[g as usize]
    }
    fn node_of_ref(&self, r: &ObjectRef) -> Option<Node> {
        let id = ref_id(r);
        match r {
            ObjectRef::PersonSlot(_) => self.slot_by_id.get(id).map(|s| Node::Person(*s)),
            ObjectRef::Office(_) => self.office_by_id.get(id).map(|o| Node::Office(*o)),
            ObjectRef::Entity(_) => self
                .group_by_id
                .get(id)
                .map(|g| Node::Group(*g))
                .or_else(|| self.polity_by_id.get(id).map(|p| Node::Polity(*p))),
            _ => None,
        }
    }
    fn node_of_id(&self, id: &str) -> Option<Node> {
        self.slot_by_id
            .get(id)
            .map(|s| Node::Person(*s))
            .or_else(|| self.office_by_id.get(id).map(|o| Node::Office(*o)))
            .or_else(|| self.group_by_id.get(id).map(|g| Node::Group(*g)))
            .or_else(|| self.polity_by_id.get(id).map(|p| Node::Polity(*p)))
    }
    fn node_id(&self, n: Node) -> &'a str {
        match n {
            Node::Office(o) => &self.m.packet.offices[o].office_id,
            Node::Group(g) => &self.m.packet.groups[g as usize].entity.id,
            Node::Person(s) => &self.m.packet.people[s as usize].person_id,
            Node::Polity(p) => &self.canon.polities[p].entity.id,
        }
    }
    fn site_label(&self, site_id: &str) -> String {
        self.labels
            .get(site_id)
            .map_or_else(|| tail(site_id).to_string(), |l| l.text.clone())
    }
    fn overlaps(&self, valid: TimeSpan) -> bool {
        valid.start_year < self.period.end_year && self.period.start_year < valid.end_year
    }

    /// Neighbours in a fixed order. A group opens its members only when `open_group`.
    fn neighbours(&self, node: Node, open_group: bool) -> Vec<Node> {
        let packet = self.m.packet;
        let mut out = Vec::new();
        match node {
            Node::Person(slot) => {
                let family = family_of(self.m.population, slot, self.at_year);
                let mut kin: Vec<u32> = family.parents;
                kin.extend(family.partners);
                kin.extend(self.partners_ever(slot));
                kin.extend(family.children);
                let mut seen = BTreeSet::new();
                out.extend(
                    kin.into_iter()
                        .filter(|s| seen.insert(*s))
                        .map(Node::Person),
                );
                for i in self.memberships_by_slot.get(&slot).into_iter().flatten() {
                    let m = &packet.memberships[*i];
                    if self.overlaps(m.valid) {
                        out.push(Node::Group(m.group));
                    }
                }
                for i in self.tenures_by_slot.get(&slot).into_iter().flatten() {
                    let t = &packet.tenures[*i];
                    if self.overlaps(t.valid) {
                        if let Some(o) = self.office_by_id.get(t.office_id.as_str()) {
                            out.push(Node::Office(*o));
                        }
                    }
                }
            }
            Node::Group(g) => {
                if open_group {
                    let mut members = members_at(packet, g, self.at_year);
                    members.sort_by_key(|m| (m.valid.start_year, m.person_slot));
                    out.extend(
                        members
                            .into_iter()
                            .take(MEMBER_CAP)
                            .map(|m| Node::Person(m.person_slot)),
                    );
                }
                let id = &packet.groups[g as usize].entity.id;
                for (o, office) in packet.offices.iter().enumerate() {
                    if &office.organization_id == id {
                        out.push(Node::Office(o));
                    }
                }
                for standing in self.standings_of(id) {
                    if let Some(j) = self.node_of_ref(&standing.jurisdiction) {
                        out.push(j);
                    }
                }
            }
            Node::Office(o) => {
                let office = &packet.offices[o];
                if let Some(g) = self.group_by_id.get(office.organization_id.as_str()) {
                    out.push(Node::Group(*g));
                }
                let mut held: Vec<&Tenure> = packet
                    .tenures
                    .iter()
                    .filter(|t| t.office_id == office.office_id && self.overlaps(t.valid))
                    .collect();
                held.sort_by_key(|t| t.valid.start_year);
                out.extend(held.iter().filter_map(|t| t.person_slot).map(Node::Person));
            }
            Node::Polity(_) => {}
        }
        out
    }
    fn partners_ever(&self, slot: u32) -> Vec<u32> {
        let unions = &self.m.population.result().union_allocations;
        self.unions_by_slot
            .get(&slot)
            .into_iter()
            .flatten()
            .flat_map(|i| unions[*i].partner_slots.iter().copied())
            .filter(|s| *s != slot)
            .collect()
    }
    fn standings_of<'s>(
        &'s self,
        subject: &'s str,
    ) -> impl Iterator<Item = &'a LegalStanding> + 's {
        self.m
            .packet
            .standings
            .iter()
            .chain(self.m.region.standings.iter())
            .filter(move |s| ref_id(&s.subject) == subject)
    }
}

// --- Entity JSON ---

/// Name fields of a person: the pool label, or the declared fallback without a pool.
fn person_name(cx: &Cx, slot: u32) -> Value {
    let packet = cx.m.packet;
    match &packet.person_names[slot as usize] {
        Some(name) => json!({
            "name_text": same_text(&name.text()),
            "name_synthetic": false,
            "name_provenance": name_provenance(name.status, DISPLAY_NAME_RULE),
        }),
        None => {
            let person = &packet.people[slot as usize];
            json!({
                "name_key": "person.fallback",
                "name_params": {
                    "tag": tag_of(&person.person_id),
                    "born": packet.lives[slot as usize].birth_year,
                    "activity": person.activity_id,
                },
                "name_synthetic": true,
            })
        }
    }
}
/// Name fields of a group: its tree rendered in each language, or the fallback.
fn group_name(cx: &Cx, g: u32) -> Result<Value, HistoryError> {
    let record = cx.group(g);
    let packet = cx.m.packet;
    let texts = match &record.entity.name_tree {
        Some(tree) => render_tree(
            tree,
            &cx.labels,
            &packet.family_surnames,
            cx.canon.world_seed,
            &packet.domain_id,
        )?,
        None => None,
    };
    Ok(match texts {
        Some(texts) => json!({
            "name_text": texts,
            "name_synthetic": false,
            "name_provenance": provenance_json(&record.name.provenance),
        }),
        None => json!({
            "name_key": "org_name.fallback",
            "name_params": {
                "kind_key": format!("name_kind.{}", record.name.kind),
                "ordinal": record.name.ordinal,
                "site": cx.site_label(&record.name.site_id),
            },
            "name_synthetic": true,
        }),
    })
}
fn with_fields(mut base: Value, fields: Value) -> Value {
    if let (Some(base), Value::Object(fields)) = (base.as_object_mut(), fields) {
        base.extend(fields);
    }
    base
}
fn person_json(cx: &Cx, slot: u32) -> Value {
    let packet = cx.m.packet;
    let table = &cx.m.population.result().candidate_table;
    let row = slot as usize;
    let person = &packet.people[row];
    let life = &packet.lives[row];
    let source = match life.source {
        SlotSource::Simulated => "lineage_simulated",
        SlotSource::Projected => "lineage_projected",
    };
    let end = life.death_year;
    let lived = TimeSpan {
        start_year: life.birth_year,
        end_year: end.unwrap_or(cx.horizon),
    };
    let observed = [table.coverage_start_year[row], table.coverage_end_year[row]];
    let names = person_name(cx, slot);
    with_fields(
        json!({
            "id": person.person_id,
            "kind": "person",
            "nature": "person",
            "agency": "individual",
            "archetype_id": "person/villager",
            "existence": span_json(lived, cx.horizon),
            "birth_year": life.birth_year,
            "death_year": end,
            "person_source": source,
            "observed_span": observed,
            "status_id": person.status_id,
            "activity_id": person.activity_id,
            "alignment_episodes": [{
                "valid": span_json(lived, cx.horizon),
                "morality_bps": person.axes.morality_bps(),
                "lawfulness_bps": person.axes.lawfulness_bps(),
                "cause_event_id": Value::Null,
                "provenance": {"status": "derived", "source": packet.plan_id, "rule_id": "alignment/v1"},
            }],
            "visibility": "open",
            "conduct_tags": [],
            "precision": "realized",
            "coverage": if life.source == SlotSource::Simulated { "complete" } else { "partial" },
            "provenance": {"status": "derived", "source": packet.plan_id,
                            "rule_id": format!("lineage/{source}")},
        }),
        names,
    )
}
fn member_series(cx: &Cx, g: u32) -> Value {
    let from = cx.period.start_year;
    let len = (cx.period.end_year - from).max(0) as usize;
    let mut counts = vec![0u32; len + 1];
    let packet = cx.m.packet;
    let start = packet.memberships.partition_point(|m| m.group < g);
    for m in packet.memberships[start..]
        .iter()
        .take_while(|m| m.group == g)
    {
        let a = (m.valid.start_year.max(from) - from).max(0) as usize;
        let b = ((m.valid.end_year.min(cx.period.end_year) - from).max(0) as usize).min(len);
        if a < b {
            counts[a] += 1;
            counts[b] = counts[b].wrapping_sub(1);
        }
    }
    let mut run = 0u32;
    let series: Vec<u32> = counts[..len]
        .iter()
        .map(|d| {
            run = run.wrapping_add(*d);
            run
        })
        .collect();
    json!({"from": from, "counts": series})
}
fn group_json(cx: &Cx, g: u32) -> Value {
    let record = cx.group(g);
    let packet = cx.m.packet;
    let entity = &record.entity;
    let names = group_name(cx, g).unwrap_or(Value::Null);
    let state = group_state(packet, g, cx.at_year);
    let realized = state
        .as_ref()
        .and_then(|s| s.member_count.value)
        .unwrap_or_else(|| members_at(packet, g, cx.at_year).len() as u64);
    with_fields(
        json!({
            "id": entity.id,
            "kind": entity.kind,
            "nature": entity.nature,
            "agency": entity.agency,
            "archetype_id": entity.archetype_id,
            "site_id": record.site_id,
            "existence": span_json(entity.existence, cx.horizon),
            "alignment_episodes": [{
                "valid": span_json(entity.existence, cx.horizon),
                "morality_bps": record.axes.morality_bps(),
                "lawfulness_bps": record.axes.lawfulness_bps(),
                "cause_event_id": Value::Null,
                "provenance": provenance_json(&entity.provenance),
            }],
            "visibility": record.visibility,
            "conduct_tags": record.conduct_tags,
            "origin": record.origin,
            "split_from": record.split_from,
            "member_count": {
                "expected": measure(Some(u64::from(record.target_members)), "persons", "expected",
                                    "aggregate", "target_members", cx.at_year),
                "realized": measure(Some(realized), "persons", "realized", "complete",
                                    "assigned_members", cx.at_year),
            },
            "member_series": member_series(cx, g),
            "precision": "realized",
            "coverage": "complete",
            "provenance": provenance_json(&entity.provenance),
        }),
        names,
    )
}
fn polity_json(cx: &Cx, p: usize) -> Value {
    let polity = &cx.canon.polities[p];
    let entity = &polity.entity;
    let names = match cx.labels.get(&entity.id) {
        Some(label)
            if let Some(texts) = polity_name(cx.canon, &cx.labels, polity).ok().flatten() =>
        {
            json!({
                "name_text": texts,
                "name_synthetic": false,
                "name_provenance": name_provenance(label.status, DISPLAY_NAME_RULE),
            })
        }
        Some(label) => json!({
            "name_key": "polity.fallback",
            "name_params": {"cell": label.text},
            "name_synthetic": false,
            "name_provenance": name_provenance(label.status, DISPLAY_NAME_RULE),
        }),
        None => json!({
            "name_key": "polity.fallback",
            "name_params": {"cell": tail(&polity.cell_id)},
            "name_synthetic": true,
        }),
    };
    with_fields(
        json!({
            "id": entity.id,
            "kind": entity.kind,
            "nature": entity.nature,
            "agency": entity.agency,
            "archetype_id": entity.archetype_id,
            "existence": span_json(entity.existence, cx.horizon),
            "alignment_episodes": [],
            "visibility": "open",
            "conduct_tags": [],
            "precision": "realized",
            "coverage": "complete",
            "provenance": provenance_json(&entity.provenance),
        }),
        names,
    )
}
fn office_json(cx: &Cx, o: usize) -> Value {
    let office: &OfficeDefinition = &cx.m.packet.offices[o];
    json!({
        "id": office.office_id,
        "kind": "office",
        "name_key": format!("office_role.{}", office.role_key),
        "organization_id": office.organization_id,
        "site_id": office.site_id,
        "collective": office.collective,
        "seats": office.seats,
        "appointment_rule": office.appointment_rule,
        "term_years": office.term_years,
        "valid": span_json(office.valid, cx.horizon),
        "observed_from": office.observed_from,
        "provenance": provenance_json(&office.provenance),
    })
}
fn tenure_json(cx: &Cx, t: &Tenure) -> Value {
    let proof = &t.selection_proof;
    json!({
        "tenure_id": t.tenure_id,
        "office_id": t.office_id,
        "person_id": t.person_id,
        "person_name": t.person_slot.map(|slot| person_name(cx, slot)),
        "valid": [t.valid.start_year, t.valid.end_year],
        "mode": t.mode,
        "cause_event_id": t.cause_event_id,
        "closed_by": t.closed_by,
        "selection_proof": {
            "summary_key": format!("selection_binding.{}", serde_json::to_value(proof.binding)
                .ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()),
            "rule": proof.rule,
            "binding": proof.binding,
            "year": proof.year,
            "vacancy_ordinal": proof.vacancy_ordinal,
            "draw_ordinal": proof.draw_ordinal,
            "candidates": proof.candidates,
            "total_weight": proof.total_weight,
            "selected_weight": proof.selected_weight,
            "fits": proof.fits,
            "alignment_fit": proof.alignment_fit,
        },
        "horizon_open": t.closed_by.is_none() && t.valid.end_year >= cx.horizon,
    })
}
fn relation_json(r: &SocialRelation, cx: &Cx) -> Value {
    json!({
        "id": r.id,
        "subject": ref_id(&r.subject),
        "predicate": r.predicate,
        "object": ref_id(&r.object),
        "valid": span_json(r.valid, cx.horizon),
        "weight_bps": r.weight_bps,
        "cause_event_id": r.cause_event_id,
        "end_cause_event_id": r.end_cause_event_id,
        "provenance": provenance_json(&r.provenance),
        "rule_id": r.rule_id,
    })
}
fn standing_json(s: &LegalStanding, cx: &Cx) -> Value {
    json!({
        "id": s.id,
        "subject": ref_id(&s.subject),
        "jurisdiction": ref_id(&s.jurisdiction),
        "status": s.status,
        "valid": span_json(s.valid, cx.horizon),
        "cause_event_id": s.cause_event_id,
        "end_cause_event_id": s.end_cause_event_id,
        "provenance": provenance_json(&s.provenance),
        "rule_id": s.rule_id,
    })
}
fn event_json(e: &HistoryEvent, kept: &BTreeSet<&str>) -> Value {
    let end = e.end_year.map_or(e.year, |end| (end - 1).max(e.year));
    json!({
        "id": e.id,
        "year": e.year,
        "end_year": end,
        "kind": e.kind,
        "phase": e.phase,
        "label_key": format!("event_kind.{}", e.kind),
        "cause_ids": e.causes,
        "subject_ids": e.participants.iter().map(ref_id)
            .filter(|id| kept.contains(id)).collect::<Vec<_>>(),
        "provenance": provenance_json(&e.provenance),
    })
}

// --- Projection ---

fn wants(query: &ViewQuery, layer: &str) -> bool {
    query.include.is_empty() || query.include.iter().any(|l| l == layer)
}
/// Opaque cursor: position in the merged item list, bound to the query that made it.
fn cursor_for(query: &ViewQuery, position: usize) -> String {
    let mut bare = query.clone();
    bare.cursor = None;
    let text = serde_json::to_string(&bare).unwrap_or_default();
    format!("c1.{position}.{:016x}", fnv64(&text))
}
fn read_cursor(query: &ViewQuery, domain: &str) -> Result<usize, HistoryError> {
    let Some(cursor) = &query.cursor else {
        return Ok(0);
    };
    let parts: Vec<&str> = cursor.split('.').collect();
    let position = parts
        .get(1)
        .and_then(|p| p.parse::<usize>().ok())
        .filter(|_| parts.len() == 3 && parts[0] == "c1");
    match position {
        Some(p) if cursor == &cursor_for(query, p) => Ok(p),
        _ => Err(fail("query.bad_cursor", domain, None)),
    }
}

fn pick_domain<'a>(
    canon: &GeneralCanon,
    mats: &[Materialized<'a>],
    query: &ViewQuery,
) -> Result<(Materialized<'a>, Option<String>), HistoryError> {
    let (domain_id, site) = if let Some(site) = &query.scope.site_id {
        let d = canon
            .domain_of_site(site)
            .ok_or_else(|| fail("query.unknown_scope", site, None))?;
        (d.domain_id.clone(), Some(site.clone()))
    } else if let Some(d) = &query.scope.domain_id {
        (d.clone(), None)
    } else if let Some(id) = query.scope.ids.first() {
        let d = domain_of_id(canon, id).ok_or_else(|| fail("query.unknown_scope", id, None))?;
        (d, None)
    } else {
        return Err(fail("query.unknown_scope", "scope", None));
    };
    let m = mats
        .iter()
        .find(|m| m.packet.domain_id == domain_id)
        .copied()
        .ok_or_else(|| fail("query.packet_not_materialized", &domain_id, None))?;
    Ok((m, site))
}

/// The bounded view of a scope at a date (§14.3). Pure: the same materialized packets
/// and the same query always give the same bytes.
pub fn project_view<'a>(
    canon: &'a GeneralCanon,
    mats: &[Materialized<'a>],
    query: &ViewQuery,
) -> Result<HistoryView, HistoryError> {
    let (m, mut site) = pick_domain(canon, mats, query)?;
    let timeline = canon.timeline;
    let at_year = query.at_year.unwrap_or(timeline.start_year);
    let period = query.period.unwrap_or(timeline);
    if !timeline.contains(at_year) || period.validate(false).is_err() || !timeline.encloses(period)
    {
        return Err(fail(
            "query.year_out_of_range",
            &canon.canon_id,
            Some(at_year),
        ));
    }
    let packet = m.packet;
    let cx = Cx::new(canon, m, at_year, period)?;

    // Seeds: the offices and organizations of a site, or the entities asked for.
    let focus: Vec<Node> = query
        .scope
        .ids
        .iter()
        .map(|id| {
            cx.node_of_id(id)
                .ok_or_else(|| fail("query.unknown_id", id, Some(at_year)))
        })
        .collect::<Result<_, _>>()?;
    let mut seeds: Vec<Node> = Vec::new();
    if focus.is_empty() {
        let at_site = |s: &str| site.as_deref().is_none_or(|x| x == s);
        let kinds = ["institution", "association", "community"];
        let mut orgs: Vec<u32> = (0..packet.groups.len() as u32)
            .filter(|g| {
                let r = cx.group(*g);
                at_site(&r.site_id)
                    && kinds.contains(&r.entity.kind.as_str())
                    && cx.overlaps(r.entity.existence)
            })
            .collect();
        orgs.sort_by_key(|g| {
            let r = cx.group(*g);
            (
                kinds.iter().position(|k| *k == r.entity.kind).unwrap_or(9),
                r.entity.id.clone(),
            )
        });
        orgs.truncate(query.limits.groups);
        let org_ids: BTreeSet<&str> = orgs
            .iter()
            .map(|g| cx.group(*g).entity.id.as_str())
            .collect();
        // Offices of the organizations shown: a head of a friend circle is not a civic charge.
        let mut offices: Vec<usize> = (0..packet.offices.len())
            .filter(|o| {
                let office = &packet.offices[*o];
                at_site(&office.site_id) && org_ids.contains(office.organization_id.as_str())
            })
            .collect();
        offices.sort_by_key(|o| {
            let office = &packet.offices[*o];
            (
                office.priority,
                office.role_key != "mayor",
                office.office_id.clone(),
            )
        });
        if wants(query, "offices") {
            seeds.extend(offices.iter().copied().map(Node::Office));
        }
        if wants(query, "organizations") {
            seeds.extend(orgs.iter().copied().map(Node::Group));
        }
        if wants(query, "people") {
            // The mayor over the whole window, every other charge at the date asked.
            let mut holders: Vec<(i32, i32, u32)> = packet
                .tenures
                .iter()
                .filter(|t| {
                    let Some(o) = cx.office_by_id.get(t.office_id.as_str()) else {
                        return false;
                    };
                    offices.contains(o)
                        && cx.overlaps(t.valid)
                        && (packet.offices[*o].role_key == "mayor" || t.valid.contains(at_year))
                })
                .filter_map(|t| {
                    let distance = if t.valid.contains(at_year) {
                        0
                    } else {
                        (t.valid.start_year - at_year)
                            .abs()
                            .min((t.valid.end_year - at_year).abs())
                    };
                    t.person_slot.map(|s| (distance, t.valid.start_year, s))
                })
                .collect();
            holders.sort();
            seeds.extend(holders.into_iter().map(|(_, _, s)| Node::Person(s)));
        }
    } else {
        seeds.extend(&focus);
    }

    // Breadth-first expansion in a fixed order.
    let depth = match (query.detail, focus.is_empty()) {
        (Detail::Summary, _) => 0,
        (Detail::Local, _) => 1,
        (Detail::Extended, _) => 2,
    };
    let mut order: Vec<Node> = Vec::new();
    let mut seen: BTreeSet<Node> = BTreeSet::new();
    for s in &seeds {
        if seen.insert(*s) {
            order.push(*s);
        }
    }
    let mut frontier: Vec<Node> = order.clone();
    for _ in 0..depth {
        let mut next = Vec::new();
        for node in &frontier {
            let open_group = focus.contains(node);
            // The default site view never fans a group out into its members.
            let expandable = !matches!(node, Node::Group(_) | Node::Office(_))
                || open_group
                || !focus.is_empty();
            if !expandable {
                continue;
            }
            for n in cx.neighbours(*node, open_group) {
                if seen.insert(n) {
                    order.push(n);
                    next.push(n);
                }
            }
        }
        frontier = next;
    }
    // Jurisdictions of included groups, so that a standing always has both ends.
    let mut extra = Vec::new();
    for node in &order {
        if let Node::Group(g) = node {
            let id = cx.group(*g).entity.id.as_str();
            for s in cx.standings_of(id) {
                if let Some(j) = cx.node_of_ref(&s.jurisdiction) {
                    extra.push(j);
                }
            }
        }
    }
    for n in extra {
        if seen.insert(n) {
            order.push(n);
        }
    }
    if site.is_none() {
        site = focus.iter().find_map(|n| match n {
            Node::Group(g) => Some(cx.group(*g).site_id.clone()),
            Node::Office(o) => Some(packet.offices[*o].site_id.clone()),
            Node::Person(s) => m
                .population
                .residences_of(*s)
                .iter()
                .find(|r| {
                    let (a, b) = state_span(
                        r.valid.start_year,
                        r.start_phase,
                        r.valid.end_year,
                        r.end_phase,
                    );
                    a <= at_year && at_year < b
                })
                .map(|r| r.site_id.clone()),
            Node::Polity(_) => None,
        });
    }

    // Relations among the retained nodes, each with its rank.
    let rank: BTreeMap<Node, usize> = order.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let mut relations: Vec<(usize, Value)> = Vec::new();
    let mut relation_ids: BTreeSet<String> = BTreeSet::new();
    let mut push_relation = |a: Node, b: Node, value: Value, ids: &mut BTreeSet<String>| {
        let id = value["id"].as_str().unwrap_or_default().to_string();
        if rank.contains_key(&a) && rank.contains_key(&b) && ids.insert(id) {
            relations.push((rank[&a].max(rank[&b]), value));
        }
    };
    let result = m.population.result();
    let table = &result.candidate_table;
    let derived = Provenance {
        status: super::model::ProvenanceStatus::Derived,
        source_ref: packet.plan_id.clone(),
        rule_id: "lineage/table".into(),
    };
    let person_ref =
        |slot: u32| ObjectRef::PersonSlot(packet.people[slot as usize].person_id.clone());
    for node in &order {
        let Node::Person(slot) = *node else { continue };
        let row = slot as usize;
        for (parent, label) in [(table.mother_slot[row], 0u32), (table.father_slot[row], 1)] {
            let Some(parent) = parent else { continue };
            let (subject, object) = (person_ref(parent), person_ref(slot));
            let start = table.birth_year[row];
            if let Ok(id) = SemanticAddress::relation(
                &cx.owner,
                Predicate::ParentOf,
                &subject,
                &object,
                start,
                label,
            ) {
                let relation = SocialRelation {
                    id: id.encode(),
                    subject,
                    predicate: Predicate::ParentOf,
                    object,
                    valid: TimeSpan {
                        start_year: start,
                        end_year: cx.horizon,
                    },
                    role_id: None,
                    weight_bps: Some(5000),
                    time_cost_bps: None,
                    cause_event_id: None,
                    end_cause_event_id: None,
                    provenance: derived.clone(),
                    rule_id: "lineage/parentage".into(),
                };
                push_relation(
                    Node::Person(parent),
                    *node,
                    relation_json(&relation, &cx),
                    &mut relation_ids,
                );
            }
        }
        for i in cx.unions_by_slot.get(&slot).into_iter().flatten() {
            let union = &result.union_allocations[*i];
            let Some(other) = union.partner_slots.iter().copied().find(|s| *s != slot) else {
                continue;
            };
            if other < slot {
                continue;
            }
            let (subject, object) = (person_ref(slot), person_ref(other));
            if let Ok(id) = SemanticAddress::relation(
                &cx.owner,
                Predicate::UnionWith,
                &subject,
                &object,
                union.valid.start_year,
                0,
            ) {
                let relation = SocialRelation {
                    id: id.encode(),
                    subject,
                    predicate: Predicate::UnionWith,
                    object,
                    valid: union.valid,
                    role_id: None,
                    weight_bps: Some(8000),
                    time_cost_bps: None,
                    cause_event_id: None,
                    end_cause_event_id: None,
                    provenance: derived.clone(),
                    rule_id: "lineage/union".into(),
                };
                push_relation(
                    *node,
                    Node::Person(other),
                    relation_json(&relation, &cx),
                    &mut relation_ids,
                );
            }
        }
        for i in cx.memberships_by_slot.get(&slot).into_iter().flatten() {
            let membership: &Membership = &packet.memberships[*i];
            if !cx.overlaps(membership.valid) {
                continue;
            }
            if let Ok(r) = membership_relation(packet, membership) {
                push_relation(
                    *node,
                    Node::Group(membership.group),
                    relation_json(&r, &cx),
                    &mut relation_ids,
                );
            }
        }
    }
    for r in &packet.relations {
        if let (Some(a), Some(b)) = (cx.node_of_ref(&r.subject), cx.node_of_ref(&r.object)) {
            if cx.overlaps(r.valid) {
                push_relation(a, b, relation_json(r, &cx), &mut relation_ids);
            }
        }
    }
    for node in &order {
        if let Node::Office(o) = *node {
            let office = &packet.offices[o];
            if let Some(g) = cx.group_by_id.get(office.organization_id.as_str()) {
                let (subject, object) = (
                    ObjectRef::Office(office.office_id.clone()),
                    ObjectRef::Entity(office.organization_id.clone()),
                );
                if let Ok(id) = SemanticAddress::relation(
                    &cx.owner,
                    Predicate::ComponentOf,
                    &subject,
                    &object,
                    office.valid.start_year,
                    0,
                ) {
                    let relation = SocialRelation {
                        id: id.encode(),
                        subject,
                        predicate: Predicate::ComponentOf,
                        object,
                        valid: office.valid,
                        role_id: None,
                        weight_bps: None,
                        time_cost_bps: None,
                        cause_event_id: None,
                        end_cause_event_id: None,
                        provenance: office.provenance.clone(),
                        rule_id: "office/organization".into(),
                    };
                    push_relation(
                        *node,
                        Node::Group(*g),
                        relation_json(&relation, &cx),
                        &mut relation_ids,
                    );
                }
            }
        }
    }
    let mut ordinals: BTreeMap<String, u32> = BTreeMap::new();
    for link in &packet.links {
        let (a, b) = (Node::Person(link.from_slot), Node::Person(link.to_slot));
        if !(rank.contains_key(&a) && rank.contains_key(&b)) || !cx.overlaps(link.valid) {
            continue;
        }
        let (subject, object) = (person_ref(link.from_slot), person_ref(link.to_slot));
        let key = format!(
            "{}|{}|{}",
            ref_id(&subject),
            ref_id(&object),
            link.valid.start_year
        );
        let ordinal = ordinals.entry(key).or_insert(0);
        if let Ok(id) = SemanticAddress::relation(
            &cx.owner,
            link.predicate,
            &subject,
            &object,
            link.valid.start_year,
            *ordinal,
        ) {
            *ordinal += 1;
            let relation = SocialRelation {
                id: id.encode(),
                subject,
                predicate: link.predicate,
                object,
                valid: link.valid,
                role_id: None,
                weight_bps: Some(6000),
                time_cost_bps: None,
                cause_event_id: None,
                end_cause_event_id: None,
                provenance: cx.group(link.group).entity.provenance.clone(),
                rule_id: "social/friend_circle_link".into(),
            };
            push_relation(a, b, relation_json(&relation, &cx), &mut relation_ids);
        }
    }
    relations.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1["id"].as_str().cmp(&b.1["id"].as_str()))
    });

    // Merged items: a node, then the relations whose last endpoint it is.
    enum Item {
        Node(Node),
        Relation(usize),
    }
    let mut items: Vec<Item> = Vec::with_capacity(order.len() + relations.len());
    let mut next_relation = 0usize;
    for (i, node) in order.iter().enumerate() {
        items.push(Item::Node(*node));
        while next_relation < relations.len() && relations[next_relation].0 == i {
            items.push(Item::Relation(next_relation));
            next_relation += 1;
        }
    }
    let start = read_cursor(query, &packet.domain_id)?;
    if start > items.len() {
        return Err(fail("query.bad_cursor", &packet.domain_id, None));
    }
    let (mut people_left, mut relations_left) = (query.limits.people, query.limits.relations);
    let mut end = start;
    while end < items.len() {
        match &items[end] {
            Item::Node(Node::Person(_)) => {
                if people_left == 0 {
                    break;
                }
                people_left -= 1;
            }
            Item::Relation(_) => {
                if relations_left == 0 {
                    break;
                }
                relations_left -= 1;
            }
            Item::Node(_) => {}
        }
        end += 1;
    }
    let cursor = (end < items.len()).then(|| cursor_for(query, end));

    let mut entities = Vec::new();
    let mut page_nodes: Vec<Node> = Vec::new();
    let mut page_relations = Vec::new();
    for item in &items[start..end] {
        match item {
            Item::Node(n) => {
                page_nodes.push(*n);
                entities.push(match n {
                    Node::Person(s) => person_json(&cx, *s),
                    Node::Group(g) => group_json(&cx, *g),
                    Node::Polity(p) => polity_json(&cx, *p),
                    Node::Office(_) => continue,
                });
            }
            Item::Relation(i) => page_relations.push(relations[*i].1.clone()),
        }
    }
    let offices: Vec<Value> = page_nodes
        .iter()
        .filter_map(|n| match n {
            Node::Office(o) => Some(office_json(&cx, *o)),
            _ => None,
        })
        .collect();
    let node_ids: BTreeSet<&str> = order.iter().map(|n| cx.node_id(*n)).collect();
    let page_ids: BTreeSet<&str> = page_nodes.iter().map(|n| cx.node_id(*n)).collect();

    // Tenures and standings of the nodes of this page.
    let tenures: Vec<Value> = packet
        .tenures
        .iter()
        .filter(|t| cx.overlaps(t.valid) && page_ids.contains(t.office_id.as_str()))
        .map(|t| tenure_json(&cx, t))
        .collect();
    let mut standing_ids = BTreeSet::new();
    let legal_standings: Vec<Value> = packet
        .standings
        .iter()
        .chain(m.region.standings.iter())
        .filter(|s| {
            cx.overlaps(s.valid)
                && page_ids.contains(ref_id(&s.subject))
                && node_ids.contains(ref_id(&s.jurisdiction))
        })
        .filter(|s| standing_ids.insert(s.id.clone()))
        .map(|s| standing_json(s, &cx))
        .collect();

    // Events: those that touch the page, those cited, and the crises of the canon.
    let mut event_pool: BTreeMap<&str, &HistoryEvent> = BTreeMap::new();
    for e in canon
        .events
        .iter()
        .chain(m.region.events.iter())
        .chain(packet.events.iter())
    {
        event_pool.insert(&e.id, e);
    }
    let mut wanted: BTreeSet<&str> = BTreeSet::new();
    for e in event_pool.values() {
        let touches = e.participants.iter().any(|p| page_ids.contains(ref_id(p)));
        let crisis = matches!(e.kind.as_str(), "war" | "epidemic" | "famine" | "treaty")
            && e.end_year.is_none_or(|end| end > period.start_year)
            && e.year < period.end_year;
        if (touches || crisis)
            && e.year < period.end_year
            && e.end_year.unwrap_or(e.year + 1) > period.start_year
        {
            wanted.insert(&e.id);
        }
    }
    let mut cited_ids: Vec<&str> = Vec::new();
    for t in packet
        .tenures
        .iter()
        .filter(|t| page_ids.contains(t.office_id.as_str()))
    {
        cited_ids.extend(t.cause_event_id.as_deref());
    }
    for v in &legal_standings {
        for k in ["cause_event_id", "end_cause_event_id"] {
            cited_ids.extend(v[k].as_str());
        }
    }
    for v in &page_relations {
        cited_ids.extend(v["cause_event_id"].as_str());
    }
    for id in cited_ids {
        if let Some((key, _)) = event_pool.get_key_value(id) {
            wanted.insert(key);
        }
    }
    // Close under causes, within a fixed depth.
    let mut frontier: Vec<&str> = wanted.iter().copied().collect();
    for _ in 0..CAUSE_DEPTH {
        let mut next = Vec::new();
        for id in frontier {
            for cause in event_pool
                .get(id)
                .map(|e| e.causes.as_slice())
                .unwrap_or_default()
            {
                if let Some((key, _)) = event_pool.get_key_value(cause.as_str()) {
                    if wanted.insert(key) {
                        next.push(*key);
                    }
                }
            }
        }
        frontier = next;
    }
    let mut chosen: Vec<&HistoryEvent> = wanted
        .iter()
        .filter_map(|id| event_pool.get(id).copied())
        .collect();
    chosen.sort_by_key(|e| ((e.year - at_year).abs(), e.id.clone()));
    let events_total = chosen.len();
    chosen.truncate(EVENT_CAP);
    chosen.sort_by(|a, b| a.year.cmp(&b.year).then_with(|| a.id.cmp(&b.id)));
    let mut kept_ids: BTreeSet<&str> = page_ids.clone();
    kept_ids.extend(canon.polities.iter().map(|p| p.entity.id.as_str()));
    let events: Vec<Value> = if wants(query, "events") {
        chosen.iter().map(|e| event_json(e, &kept_ids)).collect()
    } else {
        Vec::new()
    };

    let thresholds_person = thresholds_json(&packet.person_thresholds);
    let thresholds_org = thresholds_json(&packet.organization_thresholds);
    let (measures, balances) = if wants(query, "measures") {
        measures_for(&cx, site.as_deref())
    } else {
        (Vec::new(), json!({"flows": [], "conduct": []}))
    };
    let overlay = if wants(query, "overlay") {
        overlay_for(&cx, mats, &focus)
    } else {
        Value::Null
    };
    let default_office = site
        .as_deref()
        .and_then(|s| super::offices::mayor_office(packet, s))
        .map(|o| o.office_id.clone());
    let atlas = &canon.atlas;
    let domain_total = canon.domains.len();
    let entities_total = order.len();
    let returned_nodes = page_nodes.len();
    let returned_edges = page_relations.len();
    // Nothing is left to load: no page follows and no event was cut.
    let complete = cursor.is_none() && events_total <= EVENT_CAP;
    Ok(HistoryView {
        schema: VIEW_SCHEMA.into(),
        canon_id: canon.canon_id.clone(),
        view_fingerprint: String::new(),
        provenance: json!({
            "status": atlas.provenance.status,
            "source": atlas.atlas_id,
            "detail_key": "view.provenance.detail",
            "plan_id": packet.plan_id,
            "result_fingerprint": packet.result_fingerprint,
            "rule_id": packet.rule_id,
        }),
        period: json!({"from": period.start_year, "to": period.end_year - 1}),
        timeline: json!({"from": timeline.start_year, "to": timeline.end_year - 1}),
        scope: json!({
            "site_id": site,
            "domain_id": packet.domain_id,
            "name": site.as_deref().map(tail).unwrap_or_default(),
            "ids": query.scope.ids,
        }),
        at_year,
        observed_start: timeline.start_year,
        default_selection: json!({"office_id": default_office, "year": at_year}),
        coverage: if domain_total > 0 && mats.len() == domain_total {
            "complete"
        } else {
            "partial"
        }
        .into(),
        coverage_detail: json!({
            "domains_total": domain_total,
            "domains_materialized": mats.len(),
            "population": "assigned_members",
        }),
        limits: json!({
            "max_nodes": MAX_NODES,
            "max_edges": MAX_EDGES,
            "page": {"people": query.limits.people, "relations": query.limits.relations,
                      "groups": query.limits.groups},
            "returned_nodes": returned_nodes,
            "returned_edges": returned_edges,
            "total_nodes": entities_total,
            "total_edges": relations.len(),
            "events_total": events_total,
            "events_returned": events.len(),
            "complete": complete,
            "cursor": cursor,
        }),
        conduct_thresholds: json!({"person": thresholds_person, "organization": thresholds_org}),
        entities,
        offices,
        relations: page_relations,
        legal_standings,
        events,
        tenures,
        measures,
        balances,
        overlay,
    })
}

// --- Measures and balances ---

fn measures_for(cx: &Cx, site: Option<&str>) -> (Vec<Value>, Value) {
    let m = cx.m;
    let result = m.population.result();
    let mut measures = Vec::new();
    let mut flows = Vec::new();
    let mut conduct = Vec::new();
    let grid: Vec<i32> = marker_dates(cx.canon.timeline)
        .into_iter()
        .filter(|y| cx.period.contains(*y))
        .collect();
    if let Some(site) = site {
        let census: BTreeMap<i32, &super::population::AnnualCensus> = result
            .annual_censuses
            .iter()
            .filter(|c| c.site_id == site)
            .map(|c| (c.year, c))
            .collect();
        let series = |source: Option<&BTreeMap<i32, u64>>, basis: &str| -> Vec<Value> {
            grid.iter()
                .filter_map(|y| {
                    let expected = source?.get(y).copied()?;
                    Some(json!({
                        "at_year": y,
                        "expected": measure(Some(expected), "inhabitants", "expected", "aggregate", basis, *y),
                        "realized": census.get(y).map(|c| measure(Some(c.population), "inhabitants",
                            "realized", "complete", "simulated_domain", *y)),
                    }))
                })
                .collect()
        };
        measures.push(json!({
            "id": "m/population",
            "label_key": "measure.population",
            "note_key": "measure.population.note",
            "domain": site,
            "unit": "inhabitants",
            "series": series(m.region.population_expected_after_impacts.get(site), "forecast_after_impacts"),
        }));
        measures.push(json!({
            "id": "m/population_baseline",
            "label_key": "measure.population_baseline",
            "note_key": "measure.population_baseline.note",
            "domain": site,
            "unit": "inhabitants",
            "series": series(m.region.site_baselines.get(site), "shock_free_baseline"),
        }));
        for (year, c) in &census {
            if !cx.period.contains(*year) {
                continue;
            }
            if let Some(previous) = census.get(&(year - 1)) {
                flows.push(json!({
                    "year": year, "population_previous": previous.population,
                    "births": c.births, "deaths": c.deaths, "entries": c.incoming,
                    "exits": c.outgoing, "population": c.population,
                }));
            }
        }
        // Realized classes of the persons living on the site and of its groups.
        let class_names = ["lawless", "crooked", "ordinary", "upright", "saintly"];
        for year in &grid {
            let mut persons: BTreeMap<ConductClass, u64> = BTreeMap::new();
            let mut seen = BTreeSet::new();
            for r in &result.residence_index {
                if r.site_id != site {
                    continue;
                }
                let (a, b) = state_span(
                    r.valid.start_year,
                    r.start_phase,
                    r.valid.end_year,
                    r.end_phase,
                );
                if a <= *year && *year < b && seen.insert(r.person_slot) {
                    let axes = m.packet.people[r.person_slot as usize].axes;
                    *persons
                        .entry(conduct_class(axes, &m.packet.person_thresholds))
                        .or_default() += 1;
                }
            }
            let mut groups: BTreeMap<ConductClass, u64> = BTreeMap::new();
            for g in &m.packet.groups {
                if g.site_id == site && g.entity.existence.contains(*year) {
                    *groups
                        .entry(conduct_class(g.axes, &m.packet.organization_thresholds))
                        .or_default() += 1;
                }
            }
            let render = |counts: &BTreeMap<ConductClass, u64>| {
                let mut map = Map::new();
                for name in class_names {
                    let n: u64 = counts
                        .iter()
                        .filter(|(c, _)| {
                            serde_json::to_value(c)
                                .ok()
                                .and_then(|v| v.as_str().map(str::to_owned))
                                .as_deref()
                                == Some(name)
                        })
                        .map(|(_, n)| *n)
                        .sum();
                    map.insert(name.into(), json!(n));
                }
                Value::Object(map)
            };
            conduct.push(json!({
                "year": year,
                "person": render(&persons),
                "organization": render(&groups),
            }));
        }
    }
    for (id, label, pick) in [
        ("m/affiliated", "measure.affiliated", 0usize),
        ("m/vacant_offices", "measure.vacant_offices", 1),
    ] {
        let unit = if pick == 0 { "persons" } else { "offices" };
        let series: Vec<Value> = grid
            .iter()
            .filter_map(|y| {
                let row = m.packet.balance.iter().find(|b| b.year == *y)?;
                let value = if pick == 0 { row.persons_with_affiliation } else { row.vacant_offices };
                Some(json!({
                    "at_year": y,
                    "expected": measure(None, unit, "expected", "aggregate", "not_modelled", *y),
                    "realized": measure(Some(value), unit, "realized", "complete", "assigned_members", *y),
                }))
            })
            .collect();
        measures.push(json!({
            "id": id, "label_key": label, "domain": m.packet.domain_id,
            "unit": unit, "series": series,
        }));
    }
    (measures, json!({"flows": flows, "conduct": conduct}))
}

// --- Map overlay (§15.2) ---

fn legality_by_site(cx: &Cx, year: i32) -> BTreeMap<String, Value> {
    let packet = cx.m.packet;
    let mut status_of: BTreeMap<&str, LegalStatus> = BTreeMap::new();
    for s in packet.standings.iter().chain(cx.m.region.standings.iter()) {
        if s.valid.contains(year) {
            let worse = status_of
                .get(ref_id(&s.subject))
                .is_none_or(|old| s.status > *old);
            if worse {
                status_of.insert(ref_id(&s.subject), s.status);
            }
        }
    }
    let mut out: BTreeMap<String, [u64; 3]> = BTreeMap::new();
    for g in &packet.groups {
        if !g.entity.existence.contains(year) {
            continue;
        }
        let slot = out.entry(g.site_id.clone()).or_default();
        match status_of.get(g.entity.id.as_str()) {
            Some(LegalStatus::Tolerated) => slot[1] += 1,
            Some(LegalStatus::Proscribed) => slot[2] += 1,
            _ => slot[0] += 1,
        }
    }
    out.into_iter()
        .map(|(site, c)| {
            let total: u64 = c.iter().sum();
            let share = (c[2] * 10_000).checked_div(total).unwrap_or(0);
            (
                site,
                json!({"value": share, "licit": c[0], "tolerated": c[1], "proscribed": c[2]}),
            )
        })
        .collect()
}
fn overlay_for(cx: &Cx, mats: &[Materialized<'_>], focus: &[Node]) -> Value {
    let atlas: &HistoryAtlas = &cx.canon.atlas;
    let year = cx.at_year;
    let mut materialized_sites: BTreeMap<&str, &Materialized> = BTreeMap::new();
    for m in mats {
        for d in cx
            .canon
            .domains
            .iter()
            .filter(|d| d.domain_id == m.packet.domain_id)
        {
            for s in &d.site_ids {
                materialized_sites.insert(s.as_str(), m);
            }
        }
    }
    let root = atlas.cells.iter().find(|c| c.parent_id.is_none());
    let frame = atlas
        .settlements
        .first()
        .map(|s| s.coordinate_frame.clone());
    let legality = legality_by_site(cx, year);
    let mut sites = Vec::new();
    let mut channels: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    let channel_names: Vec<String> =
        cx.m.region
            .social
            .catalogue
            .engine_defaults
            .influence_profiles
            .keys()
            .cloned()
            .chain([LEGALITY_CHANNEL.to_string()])
            .collect();
    for settlement in &atlas.settlements {
        let episode = settlement.episode_at(year);
        let census = materialized_sites
            .get(settlement.site_id.as_str())
            .and_then(|m| {
                m.population
                    .result()
                    .annual_censuses
                    .iter()
                    .find(|c| c.site_id == settlement.site_id && c.year == year)
            });
        let controller = cx.canon.controller_at(&settlement.site_id, year);
        sites.push(json!({
            "site_id": settlement.site_id,
            "label": cx.site_label(&settlement.site_id),
            "label_provenance": cx.labels.get(&settlement.site_id)
                .map(|l| name_provenance(l.status, DISPLAY_NAME_RULE)),
            "cell_id": settlement.cell_id,
            "x_mm": settlement.x_mm,
            "y_mm": settlement.y_mm,
            "foundation_year": settlement.foundation_year,
            "state": episode.map(|e| e.state),
            "capacity": episode.map(|e| e.capacity),
            "tags": settlement.site_tags,
            "population": census.map(|c| c.population),
            "population_expected_max": settlement.population_expected_max,
            "controller_id": controller.map(|p| p.entity.id.clone()),
            "materialized": materialized_sites.contains_key(settlement.site_id.as_str()),
        }));
        let Some(m) = materialized_sites.get(settlement.site_id.as_str()) else {
            continue;
        };
        for name in &channel_names {
            let entry = if name == LEGALITY_CHANNEL {
                legality.get(&settlement.site_id).cloned()
            } else {
                influence_at(m.region, m.packet, &settlement.site_id, name, year)
                    .ok()
                    .map(|c| {
                        let total: u64 = c.iter().map(|x| x.value).sum();
                        json!({"value": total, "sources": c.len()})
                    })
            };
            if let Some(entry) = entry {
                channels
                    .entry(name.clone())
                    .or_default()
                    .insert(settlement.site_id.clone(), entry);
            }
        }
    }
    let routes: Vec<Value> = atlas
        .mobility_edges
        .iter()
        .filter(|e| e.valid.contains(year))
        .map(|e| {
            json!({"edge_id": e.edge_id, "from": e.from_site_id, "to": e.to_site_id,
                   "travel_days": e.travel_days, "mode": e.mode, "bidirectional": e.bidirectional})
        })
        .collect();
    // Contacts keep no position outside the map: the site end is drawn, the outside
    // context is only named (§15.2).
    let contacts: Vec<Value> = atlas
        .external_contacts
        .iter()
        .filter(|c| c.valid.contains(year))
        .map(|c| {
            let mut sites_end = Vec::new();
            let mut outside = Vec::new();
            for end in &c.ends {
                match end {
                    ContactEnd::Site(s) => sites_end.push(json!({
                        "site_id": s.site_id, "external_context_id": s.external_context_id,
                        "external_profile_id": s.external_profile_id,
                        "population_expected": s.population_expected,
                        "access_edge_id": s.access_edge_id,
                    })),
                    ContactEnd::External { context_id } => outside.push(context_id.clone()),
                }
            }
            json!({"contact_id": c.contact_id, "valid": span_json(c.valid, cx.horizon),
                   "site_ends": sites_end, "outside_contexts": outside})
        })
        .collect();
    let selection = focus.iter().find_map(|n| match n {
        Node::Group(g) => Some(selection_for(cx, *g, &channel_names)),
        _ => None,
    });
    json!({
        "schema": "chronos.historical-overlay.v1",
        "coordinate_frame": frame,
        "year": year,
        "bounds": root.map(|c| json!({
            "min_x_mm": c.bounds.min_x_mm, "min_y_mm": c.bounds.min_y_mm,
            "max_x_mm": c.bounds.max_x_mm, "max_y_mm": c.bounds.max_y_mm,
        })),
        "provenance": provenance_json(&atlas.provenance),
        "sites": sites,
        "routes": routes,
        "channels": channels,
        "channel_names": channel_names,
        "external_contacts": contacts,
        "selection": selection,
    })
}
/// Where a group is present: the sites its members live on, its influence, and the
/// other chapters of the same archetype. Positions come from the atlas only.
fn selection_for(cx: &Cx, g: u32, channels: &[String]) -> Value {
    let packet = cx.m.packet;
    let record = cx.group(g);
    let mut residents: BTreeMap<String, u64> = BTreeMap::new();
    for member in members_at(packet, g, cx.at_year) {
        for r in cx.m.population.residences_of(member.person_slot) {
            let (a, b) = state_span(
                r.valid.start_year,
                r.start_phase,
                r.valid.end_year,
                r.end_phase,
            );
            if a <= cx.at_year && cx.at_year < b {
                *residents.entry(r.site_id.clone()).or_default() += 1;
            }
        }
    }
    let mut chapters: BTreeSet<String> = BTreeSet::new();
    for other in &packet.groups {
        if other.entity.archetype_id == record.entity.archetype_id
            && other.entity.existence.contains(cx.at_year)
            && other.entity.id != record.entity.id
        {
            chapters.insert(other.site_id.clone());
        }
    }
    let mut influence: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for site in &cx.m.region.social.atlas.settlements {
        for name in channels.iter().filter(|c| *c != LEGALITY_CHANNEL) {
            if let Ok(list) = influence_at(cx.m.region, packet, &site.site_id, name, cx.at_year) {
                if let Some(c) = list.iter().find(|c| c.group_id == record.entity.id) {
                    influence.entry(name.clone()).or_default().insert(
                        site.site_id.clone(),
                        json!({"value": c.value, "travel_days": c.travel_days}),
                    );
                }
            }
        }
    }
    json!({
        "group_id": record.entity.id,
        "seat_site_id": record.site_id,
        "residents_by_site": residents,
        "other_chapters": chapters,
        "influence": influence,
    })
}

// --- Dated sheet and explanation ---

fn person_sheet(cx: &Cx, slot: u32) -> Result<Value, HistoryError> {
    let packet = cx.m.packet;
    let record = person_record(packet, cx.m.population, slot)?;
    let family = family_of(cx.m.population, slot, cx.at_year);
    let id_of = |s: &u32| packet.people[*s as usize].person_id.clone();
    let memberships: Vec<Value> = cx
        .memberships_by_slot
        .get(&slot)
        .into_iter()
        .flatten()
        .map(|i| &packet.memberships[*i])
        .filter(|m| m.valid.contains(cx.at_year))
        .map(|m| {
            json!({"group_id": cx.group(m.group).entity.id, "predicate": m.predicate,
                   "valid": [m.valid.start_year, m.valid.end_year]})
        })
        .collect();
    let offices: Vec<Value> = cx
        .tenures_by_slot
        .get(&slot)
        .into_iter()
        .flatten()
        .map(|i| &packet.tenures[*i])
        .filter(|t| t.valid.contains(cx.at_year))
        .map(|t| json!({"office_id": t.office_id, "tenure_id": t.tenure_id, "mode": t.mode}))
        .collect();
    let class = conduct_class(packet.people[slot as usize].axes, &packet.person_thresholds);
    Ok(json!({
        "record": record,
        "conduct_class": class,
        "family": {
            "parents": family.parents.iter().map(id_of).collect::<Vec<_>>(),
            "children": family.children.iter().map(id_of).collect::<Vec<_>>(),
            "partners": family.partners.iter().map(id_of).collect::<Vec<_>>(),
        },
        "memberships": memberships,
        "offices": offices,
    }))
}

/// Dated sheet of one entity (§14.2). `packets_needed` names what had to be read.
pub fn entity_sheet<'a>(
    canon: &'a GeneralCanon,
    mats: &[Materialized<'a>],
    id: &str,
    at_year: i32,
) -> Result<Value, HistoryError> {
    let domain =
        domain_of_id(canon, id).ok_or_else(|| fail("query.unknown_id", id, Some(at_year)))?;
    let m = mats
        .iter()
        .find(|m| m.packet.domain_id == domain)
        .copied()
        .ok_or_else(|| fail("query.packet_not_materialized", &domain, None))?;
    if !canon.timeline.contains(at_year) {
        return Err(fail("query.year_out_of_range", id, Some(at_year)));
    }
    let cx = Cx::new(canon, m, at_year, canon.timeline)?;
    let node = cx
        .node_of_id(id)
        .ok_or_else(|| fail("query.unknown_id", id, Some(at_year)))?;
    let (kind, sheet) = match node {
        Node::Person(slot) => ("person", person_sheet(&cx, slot)?),
        Node::Group(g) => {
            let record = cx.group(g);
            let state = group_state(m.packet, g, at_year);
            let standings: Vec<Value> = cx
                .standings_of(&record.entity.id)
                .filter(|s| s.valid.contains(at_year))
                .map(|s| standing_json(s, &cx))
                .collect();
            (
                "group",
                json!({"entity": group_json(&cx, g), "state": state, "standings": standings}),
            )
        }
        Node::Office(o) => {
            let view = resolve_office(m.packet, &m.packet.offices[o].office_id, at_year)?;
            ("office", json!({"view": view}))
        }
        Node::Polity(p) => ("polity", json!({"entity": polity_json(&cx, p)})),
    };
    Ok(json!({
        "id": id,
        "kind": kind,
        "at_year": at_year,
        "packets_needed": [domain],
        "sheet": sheet,
    }))
}

/// Source, rule and dependencies of one fact (§14.2). A fact owned by a cell or the
/// world is found in the canon or the region plans; the others need their packet.
pub fn explain_fact<'a>(
    canon: &'a GeneralCanon,
    regions: &[&RegionPlan],
    mats: &[Materialized<'a>],
    fact_id: &str,
) -> Result<Value, HistoryError> {
    let address =
        SemanticAddress::decode(fact_id).map_err(|_| fail("query.unknown_id", fact_id, None))?;
    let domain = domain_of_id(canon, fact_id);
    let mat = match &domain {
        Some(d) => Some(
            mats.iter()
                .find(|m| m.packet.domain_id == *d)
                .copied()
                .ok_or_else(|| fail("query.packet_not_materialized", d, None))?,
        ),
        None => None,
    };
    let mut events: BTreeMap<&str, &HistoryEvent> = BTreeMap::new();
    for e in canon
        .events
        .iter()
        .chain(regions.iter().flat_map(|r| r.events.iter()))
    {
        events.insert(&e.id, e);
    }
    if let Some(m) = &mat {
        for e in m.packet.events.iter().chain(m.region.events.iter()) {
            events.insert(&e.id, e);
        }
    }
    let chain = |start: &[String]| -> Vec<Value> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        let mut frontier: Vec<&str> = start.iter().map(String::as_str).collect();
        for _ in 0..CAUSE_DEPTH {
            let mut next = Vec::new();
            for id in frontier {
                if let Some(e) = events.get(id) {
                    if seen.insert(id) {
                        out.push(json!({"id": e.id, "kind": e.kind, "year": e.year,
                                         "label_key": format!("event_kind.{}", e.kind)}));
                        next.extend(e.causes.iter().map(String::as_str));
                    }
                }
            }
            frontier = next;
        }
        out
    };
    let horizon = canon.timeline.end_year;
    if let Some(e) = events.get(fact_id) {
        return Ok(json!({
            "fact_id": fact_id, "kind": "event", "event_kind": e.kind, "year": e.year,
            "end_year": e.end_year.map(|v| v - 1), "phase": e.phase, "domain_id": domain,
            "rule_id": e.provenance.rule_id, "provenance": provenance_json(&e.provenance),
            "causes": chain(&e.causes),
            "participants": e.participants.iter().map(ref_id).collect::<Vec<_>>(),
            "sentence_key": "explain.event",
            "sentence_params": {"kind_key": format!("event_kind.{}", e.kind), "year": e.year},
        }));
    }
    let Some(m) = mat else {
        return Err(fail("query.unknown_id", fact_id, None));
    };
    let packet = m.packet;
    if let Some(t) = packet.tenures.iter().find(|t| t.tenure_id == fact_id) {
        let office = packet.offices.iter().find(|o| o.office_id == t.office_id);
        let causes = t.cause_event_id.iter().cloned().collect::<Vec<_>>();
        return Ok(json!({
            "fact_id": fact_id, "kind": "tenure", "domain_id": domain,
            "office_id": t.office_id, "person_id": t.person_id,
            "valid": [t.valid.start_year, t.valid.end_year], "mode": t.mode,
            "closed_by": t.closed_by,
            "rule_id": office.map(|o| o.provenance.rule_id.clone()),
            "provenance": office.map(|o| provenance_json(&o.provenance)),
            "causes": chain(&causes),
            "selection": {
                "rule": t.selection_proof.rule, "binding": t.selection_proof.binding,
                "candidates": t.selection_proof.candidates,
                "total_weight": t.selection_proof.total_weight,
                "selected_weight": t.selection_proof.selected_weight,
                "fits": t.selection_proof.fits, "alignment_fit": t.selection_proof.alignment_fit,
            },
            "constraints": office.map(|o| json!({
                "min_age": o.requirements.min_age, "max_age": o.requirements.max_age,
                "residence_site_id": o.requirements.residence_site_id,
                "jurisdiction_bound": o.requirements.jurisdiction_bound,
                "term_years": o.term_years,
            })),
            "sentence_key": "explain.tenure",
            "sentence_params": {"year": t.valid.start_year},
        }));
    }
    if let Some(s) = packet
        .standings
        .iter()
        .chain(m.region.standings.iter())
        .find(|s| s.id == fact_id)
    {
        let causes: Vec<String> = s
            .cause_event_id
            .iter()
            .chain(s.end_cause_event_id.iter())
            .cloned()
            .collect();
        return Ok(json!({
            "fact_id": fact_id, "kind": "standing", "domain_id": domain,
            "subject": ref_id(&s.subject), "jurisdiction": ref_id(&s.jurisdiction),
            "status": s.status, "valid": [s.valid.start_year, s.valid.end_year],
            "rule_id": s.rule_id, "provenance": provenance_json(&s.provenance),
            "causes": chain(&causes),
            "sentence_key": "explain.standing",
            "sentence_params": {"year": s.valid.start_year},
        }));
    }
    if let Some(r) = packet.relations.iter().find(|r| r.id == fact_id) {
        let causes: Vec<String> = r.cause_event_id.iter().cloned().collect();
        return Ok(json!({
            "fact_id": fact_id, "kind": "relation", "domain_id": domain,
            "predicate": r.predicate, "subject": ref_id(&r.subject), "object": ref_id(&r.object),
            "valid": [r.valid.start_year, r.valid.end_year],
            "rule_id": r.rule_id, "provenance": provenance_json(&r.provenance),
            "causes": chain(&causes),
            "sentence_key": "explain.relation",
            "sentence_params": {"year": r.valid.start_year},
        }));
    }
    // Memberships and kin links are read from the table: they are not stored as facts.
    if let Some(link) = address.link() {
        let cx = Cx::new(canon, m, link.start, canon.timeline)?;
        let ends = [
            ObjectRef::from_address(&link.subject).ok(),
            ObjectRef::from_address(&link.object).ok(),
        ];
        if let [Some(a), Some(b)] = ends {
            let (a, b) = (cx.node_of_ref(&a), cx.node_of_ref(&b));
            let membership = match (a, b) {
                (Some(Node::Person(s)), Some(Node::Group(g))) => cx
                    .memberships_by_slot
                    .get(&s)
                    .into_iter()
                    .flatten()
                    .map(|i| &packet.memberships[*i])
                    .find(|m| m.group == g && m.opened_year == link.start),
                _ => None,
            };
            if let Some(m) = membership {
                let causes: Vec<String> = m.closing_event_id.iter().cloned().collect();
                return Ok(json!({
                    "fact_id": fact_id, "kind": "membership", "domain_id": domain,
                    "predicate": m.predicate, "valid": [m.valid.start_year, m.valid.end_year],
                    "closed_by": m.closed_by,
                    "rule_id": "social/assignment",
                    "selection": {"vacancy_ordinal": m.vacancy_ordinal, "draw_ordinal": m.draw_ordinal,
                                   "candidates": m.candidates},
                    "causes": chain(&causes),
                    "sentence_key": "explain.membership",
                    "sentence_params": {"year": m.valid.start_year},
                }));
            }
            return Ok(json!({
                "fact_id": fact_id, "kind": "kinship", "domain_id": domain,
                "predicate": address.link().map(|_| "lineage"),
                "valid": [link.start, horizon],
                "rule_id": "lineage/table",
                "causes": [],
                "sentence_key": "explain.kinship",
                "sentence_params": {"year": link.start},
            }));
        }
    }
    Err(fail("query.unknown_id", fact_id, None))
}

/// Tenures of the mode `Vacant` carry no person; helper kept for the host's checks.
pub fn is_vacant(t: &Tenure) -> bool {
    t.mode == TenureMode::Vacant
}
