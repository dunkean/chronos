//! S4 exit proofs on a small hand-built table (§20.2). Demo fixture, world seed 11,
//! `chronos-social-2`, horizon 1300–1400. Slots are drawn from the expected baseline, so
//! the demo canon gives real group slots while only the listed people exist.
use super::catalog::HistoryCatalogue;
use super::general::{build_canon, AuthorConstraints, GeneralCanon, WorldConfig};
use super::ids::{DomainKey, SemanticAddress};
use super::model::{
    conduct_class, ConductClass, LegalStanding, LegalStatus, ObjectRef, Predicate, TimeSpan,
    Visibility,
};
use super::offices::{mayor_office, resolve_office, OfficeStatus, TenureEnd, TenureMode};
use super::population::{parse_population_result, PopulationResult};
use super::region::{plan_region, RegionPlan};
use super::social::{
    build_from_result, membership_relation, memberships_of, project_window, GroupRecord,
    MembershipEnd, SocialPacket,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const START: i32 = 1300;
const END: i32 = 1400;

fn canon() -> GeneralCanon {
    let config: WorldConfig = serde_json::from_str(include_str!(
        "../../../../fixtures/social-v2/region-demo.world.json"
    ))
    .unwrap();
    let atlas = serde_json::from_str(include_str!(
        "../../../../fixtures/social-v2/region-demo.atlas.json"
    ))
    .unwrap();
    let catalogue: HistoryCatalogue = serde_json::from_str(include_str!(
        "../../../../docs/design/chronos-social-v2/culture-bindings.v1.json"
    ))
    .unwrap();
    build_canon(&config, &atlas, &catalogue, &AuthorConstraints::default()).unwrap()
}
struct World {
    canon: GeneralCanon,
    region: RegionPlan,
}
fn world() -> World {
    let canon = canon();
    let region_id = canon.baselines.keys().next().unwrap().clone();
    let region = plan_region(&canon, &region_id).unwrap();
    World { canon, region }
}
impl World {
    fn domain(&self) -> &super::atlas::DemographicDomain {
        &self.canon.domains[0]
    }
    fn site(&self, index: usize) -> String {
        self.domain().site_ids[index].clone()
    }
    fn plan_id(&self) -> String {
        let key = DomainKey::from_id(&self.domain().domain_id).unwrap();
        SemanticAddress::plan(&key).unwrap().encode()
    }
}
#[derive(Clone)]
struct Person {
    birth: i32,
    death: Option<i32>,
    left: Option<i32>,
    site: String,
    family: usize,
    ancestry: &'static str,
}
fn adult(birth: i32, site: &str) -> Person {
    Person {
        birth,
        death: None,
        left: None,
        site: site.into(),
        family: birth as usize,
        ancestry: "human-demo",
    }
}
/// Residents of one site, with staggered lives that keep every age present.
fn village(site: &str, count: usize) -> Vec<Person> {
    (0..count)
        .map(|i| {
            let birth = 1262 + (i as i32 * 7) % 120;
            Person {
                death: Some(birth + 55 + (i as i32 * 3) % 25),
                family: i / 3,
                ..adult(birth, site)
            }
        })
        .collect()
}
/// A table closed by construction: censuses come from the same rows as residences.
fn table(world: &World, people: &[Person]) -> PopulationResult {
    let n = people.len();
    let end_of = |p: &Person| match (p.death, p.left) {
        (Some(d), _) => (d, 30, "death"),
        (None, Some(l)) => (l, 40, "departed_external"),
        _ => (END, 0, "coverage_end"),
    };
    let residences: Vec<Value> = people
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (end, phase, reason) = end_of(p);
            json!({"person_slot": i, "site_id": p.site,
                "valid": {"start_year": p.birth.max(START), "end_year": end},
                "start_phase": if p.birth < START {10} else {20},
                "end_phase": phase, "end_reason": reason})
        })
        .collect();
    let sites: BTreeSet<&str> = people.iter().map(|p| p.site.as_str()).collect();
    let mut censuses = Vec::new();
    for site in sites {
        let here: Vec<&Person> = people.iter().filter(|p| p.site == site).collect();
        for year in START..END {
            let present = |p: &&&Person| p.birth <= year && end_of(p).0 > year;
            let count = |f: &dyn Fn(&Person) -> bool| here.iter().filter(|p| f(p)).count();
            censuses.push(json!({"site_id": site, "year": year,
                "population": here.iter().filter(present).count(),
                "simulated": 0, "projected": here.iter().filter(present).count(),
                "births": if year > START {count(&|p| p.birth == year)} else {0},
                "deaths": count(&|p| p.death == Some(year)),
                "incoming": 0,
                "outgoing": count(&|p| p.death.is_none() && p.left == Some(year)),
                "precision": "realized", "coverage": "complete"}));
        }
    }
    let column = |f: &dyn Fn(usize, &Person) -> Value| -> Value {
        people.iter().enumerate().map(|(i, p)| f(i, p)).collect()
    };
    let config = &world.canon.config;
    let value = json!({
        "format": "population-result-json-1", "result_type": "PopulationResult",
        "complete": true, "coverage": "complete", "stage": "test",
        "plan_id": world.plan_id(), "plan_fingerprint": "test",
        "result_fingerprint": "test",
        "timeline": {"start_year": START, "end_year": END},
        "provider_manifest": {
            "algorithm_id": config.provider.algorithm_id,
            "projection_version": config.provider.projection_version,
            "runtime_fingerprint": "test",
            "catalogue_fingerprints": config.catalogue_fingerprints,
        },
        "candidate_table": {
            "origin_id": column(&|i, _| json!(format!("slot/{i:05}"))),
            "origin_cohort_id": column(&|_, _| json!("cohort/0")),
            "sex_ordinal": column(&|i, _| json!(i)),
            "source": column(&|_, _| json!("projected")),
            "ancestry_id": column(&|_, p| json!(p.ancestry)),
            "sex": column(&|i, _| json!(i % 2)),
            "birth_year": column(&|_, p| json!(p.birth)),
            "death_year": column(&|_, p| json!(p.death)),
            "death_site_id": column(&|_, p| json!(p.death.map(|_| p.site.clone()))),
            "coverage_start_year": column(&|_, p| json!(p.birth.max(START))),
            "coverage_end_year": column(&|_, p| json!(end_of(p).0)),
            "departure_state": column(&|_, p| json!(match end_of(p).2 {
                "death" => "dead",
                "departed_external" => "departed_external",
                _ => "resident_at_coverage_end",
            })),
            "departure_year": column(&|_, p| json!(p.left)),
            "mother_slot": column(&|_, _| Value::Null),
            "father_slot": column(&|_, _| Value::Null),
            "family_key": column(&|_, p| json!(format!("family/{}", p.family))),
            "reproduction_flags": column(&|_, _| json!(0)),
            "core_attributes": column(&|_, _| Value::Null),
        },
        "origin_cohorts": [],
        "residence_index": residences,
        "residence_offsets": (0..=n).collect::<Vec<_>>(),
        "household_episodes": [], "union_allocations": [], "birth_allocations": [],
        "death_allocations": [], "residence_transfers": [], "guardian_assignments": [],
        "parentage": [],
        "children_index": vec![Vec::<u32>::new(); n],
        "annual_censuses": censuses, "cohort_residence_counts": [],
        "diagnostics": {}, "validation_proof": {},
    });
    parse_population_result(&serde_json::to_vec(&value).unwrap()).unwrap()
}
fn packet(world: &World, people: &[Person]) -> SocialPacket {
    build_from_result(&world.region, &table(world, people), END).unwrap()
}
fn holders(packet: &SocialPacket, office_id: &str) -> Vec<(i32, i32, Option<u32>, TenureMode)> {
    packet
        .tenures
        .iter()
        .filter(|t| t.office_id == office_id)
        .map(|t| (t.valid.start_year, t.valid.end_year, t.person_slot, t.mode))
        .collect()
}

#[test]
fn mayor_is_stable_across_query_orders() {
    let world = world();
    let (a, b) = (world.site(0), world.site(1));
    let people = [village(&a, 120), village(&b, 120)].concat();
    let first = packet(&world, &people);
    let second = packet(&world, &people);
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    let ask = |p: &SocialPacket, site: &str, year| {
        let office = mayor_office(p, site).unwrap();
        resolve_office(p, &office.office_id, year).unwrap()
    };
    let forward = [ask(&first, &a, 1360), ask(&first, &b, 1360)];
    let backward = [ask(&second, &b, 1360), ask(&second, &a, 1360)];
    assert_eq!(forward[0], backward[1]);
    assert_eq!(forward[1], backward[0]);
    assert_eq!(forward[0].status, OfficeStatus::Held);
    let late_first = (ask(&first, &a, 1380), ask(&first, &a, 1340));
    let early_first = (ask(&second, &a, 1340), ask(&second, &a, 1380));
    assert_eq!(late_first.0, early_first.1);
    assert_eq!(late_first.1, early_first.0);
}

#[test]
fn direct_lookup_matches_refined_lookup() {
    let world = world();
    let site = world.site(2);
    let people = village(&site, 120);
    let packet = packet(&world, &people);
    let office = mayor_office(&packet, &site).unwrap().office_id.clone();
    let direct = resolve_office(&packet, &office, 1360).unwrap();
    // World, then region, then village: domain of the site, its anchor, its head.
    let domain = world.canon.domain_of_site(&site).unwrap();
    assert_eq!(domain.domain_id, packet.domain_id);
    let anchor = packet
        .groups
        .iter()
        .find(|g| {
            g.site_id == site
                && g.entity.kind == "institution"
                && g.target_members > 0
                && packet
                    .offices
                    .iter()
                    .any(|o| o.organization_id == g.entity.id && o.collective)
        })
        .unwrap();
    let refined = packet
        .offices
        .iter()
        .find(|o| o.organization_id == anchor.entity.id && !o.collective)
        .unwrap();
    assert_eq!(refined.office_id, office);
    let tenure = packet
        .tenures
        .iter()
        .find(|t| t.office_id == refined.office_id && t.valid.contains(1360));
    assert_eq!(direct.tenure.as_ref(), tenure);
}

#[test]
fn overlapping_windows_agree() {
    let world = world();
    let site = world.site(0);
    let people = village(&site, 150);
    let packet = packet(&world, &people);
    let wide = project_window(
        &packet,
        TimeSpan {
            start_year: START,
            end_year: END,
        },
    );
    let narrow_span = TimeSpan {
        start_year: 1350,
        end_year: 1370,
    };
    let narrow = project_window(&packet, narrow_span);
    let mut from_wide = wide.clone();
    from_wide.tenures.retain(|t| {
        t.valid.start_year < narrow_span.end_year && narrow_span.start_year < t.valid.end_year
    });
    let keys = |tenures: &[super::offices::Tenure]| -> Vec<_> {
        tenures
            .iter()
            .map(|t| (t.tenure_id.clone(), t.person_id.clone(), t.mode))
            .collect()
    };
    assert!(!narrow.tenures.is_empty());
    assert_eq!(keys(&narrow.tenures), keys(&from_wide.tenures));
    for tenure in &narrow.tenures {
        assert!(narrow_span.encloses(tenure.valid));
    }
}

#[test]
fn affiliations_do_not_add_people() {
    let world = world();
    let site = world.site(0);
    let people = village(&site, 200);
    let packet = packet(&world, &people);
    let table = table(&world, &people);
    for row in &packet.balance {
        let census: u64 = table
            .annual_censuses
            .iter()
            .filter(|c| c.year == row.year)
            .map(|c| c.population)
            .sum();
        assert_eq!(row.population, census, "{}", row.year);
        assert!(row.persons_with_affiliation <= row.population);
    }
    assert!(packet.balance.iter().any(|b| b.memberships > b.population));
    let overlapping = (0..people.len() as u32)
        .map(|slot| memberships_of(&packet, slot, 1340).len())
        .max()
        .unwrap();
    assert!(overlapping >= 3, "family, work, cult and friends overlap");
    for m in &packet.memberships {
        let p = &people[m.person_slot as usize];
        assert!(p.birth <= m.valid.start_year);
        assert!(p.death.is_none_or(|d| m.valid.end_year <= d));
        assert!(m.valid.start_year < m.valid.end_year);
    }
    let mut commitment: BTreeMap<(u32, i32), u32> = BTreeMap::new();
    for m in packet.memberships.iter().filter(|m| m.time_cost_bps > 0) {
        for year in m.valid.start_year..m.valid.end_year {
            *commitment.entry((m.person_slot, year)).or_default() += u32::from(m.time_cost_bps);
        }
    }
    assert!(commitment.values().all(|bps| *bps <= 10_000));
}

fn only_candidate(world: &World, death: Option<i32>, left: Option<i32>) -> (SocialPacket, String) {
    let site = world.site(0);
    let holder = Person {
        death,
        left,
        ..adult(1270, &site)
    };
    // Too young to ever reach the minimum age before the end of the test window.
    let packet = packet(world, &[holder, adult(1390, &site)]);
    let office = mayor_office(&packet, &site).unwrap().office_id.clone();
    (packet, office)
}

#[test]
fn death_closes_tenure_at_boundary() {
    let world = world();
    let (packet, office) = only_candidate(&world, Some(1322), None);
    let sequence = holders(&packet, &office);
    let first = packet
        .tenures
        .iter()
        .find(|t| t.office_id == office)
        .unwrap();
    assert_eq!(
        first.valid.start_year, START,
        "observed from the world start"
    );
    assert_eq!(
        first.selection_proof.binding,
        super::offices::SelectionBinding::InitialObservedBinding
    );
    assert_eq!(
        first.cause_event_id, None,
        "no election is claimed before it"
    );
    let last = sequence.iter().rev().find(|t| t.2 == Some(0)).unwrap();
    assert_eq!(last.1, 1322, "the mandate ends at the death boundary");
    let next = sequence.iter().find(|t| t.0 == 1322).unwrap();
    assert_eq!(next.3, TenureMode::Vacant, "vacancy opens at the same date");
    let closed = packet
        .tenures
        .iter()
        .find(|t| t.office_id == office && t.valid.end_year == 1322)
        .unwrap();
    assert_eq!(closed.closed_by, Some(TenureEnd::Death));
    assert_eq!(
        resolve_office(&packet, &office, 1321).unwrap().status,
        OfficeStatus::Held
    );
    assert_eq!(
        resolve_office(&packet, &office, 1322).unwrap().status,
        OfficeStatus::Vacant
    );
}

#[test]
fn departure_closes_local_tenure() {
    let world = world();
    let (packet, office) = only_candidate(&world, None, Some(1317));
    let closed = packet
        .tenures
        .iter()
        .find(|t| t.office_id == office && t.valid.end_year == 1317)
        .unwrap();
    assert_eq!(closed.closed_by, Some(TenureEnd::Departure));
    assert_eq!(
        resolve_office(&packet, &office, 1317).unwrap().status,
        OfficeStatus::Vacant
    );
}

#[test]
fn truncated_horizon_preserves_earlier_tenures() {
    let world = world();
    let site = world.site(0);
    let table = table(&world, &village(&site, 150));
    let full = build_from_result(&world.region, &table, END).unwrap();
    let span = TimeSpan {
        start_year: START,
        end_year: 1361,
    };
    let prefix = build_from_result(&world.region, &table, span.end_year).unwrap();
    assert_eq!(prefix.horizon_end, 1361);
    let (a, b) = (project_window(&full, span), project_window(&prefix, span));
    assert_eq!(a.tenures, b.tenures);
    // A later split adds groups: indices shift, group identities do not.
    let facts = |packet: &SocialPacket, window: &super::social::SocialWindow| {
        let mut rows: Vec<_> = window
            .memberships
            .iter()
            .map(|m| {
                let group = packet.groups[m.group as usize].entity.id.clone();
                (
                    group,
                    m.person_slot,
                    m.valid.start_year,
                    m.valid.end_year,
                    m.closed_by,
                )
            })
            .collect();
        rows.sort();
        rows
    };
    assert_eq!(facts(&full, &a), facts(&prefix, &b));
}

#[test]
fn proscribed_person_cannot_hold_office() {
    let mut world = world();
    let site = world.site(0);
    let people = vec![adult(1275, &site), adult(1276, &site)];
    let free = packet(&world, &people);
    let office = mayor_office(&free, &site).unwrap().office_id.clone();
    let chosen = free
        .tenures
        .iter()
        .find(|t| t.office_id == office)
        .and_then(|t| t.person_slot)
        .unwrap();
    let person = free.people[chosen as usize].person_id.clone();
    let jurisdiction = world
        .canon
        .controller_at(&site, START)
        .unwrap()
        .entity
        .id
        .clone();
    let subject = ObjectRef::PersonSlot(person);
    let object = ObjectRef::Entity(jurisdiction);
    let owner = super::ids::Owner::cell(&world.canon.canon_id, &world.region.region_id);
    let id = SemanticAddress::standing(&owner, &subject, &object, START, 0).unwrap();
    world.region.standings.push(LegalStanding {
        id: id.encode(),
        subject,
        jurisdiction: object,
        status: LegalStatus::Proscribed,
        valid: TimeSpan {
            start_year: START,
            end_year: END,
        },
        cause_event_id: None,
        end_cause_event_id: None,
        provenance: world.canon.provenance("test"),
        rule_id: "test".into(),
    });
    let barred = packet(&world, &people);
    // Control may change hands: the ban holds as long as this jurisdiction rules.
    let bans = |year| {
        world
            .canon
            .controller_at(&site, year)
            .map(|p| p.entity.id.clone())
            == world
                .canon
                .controller_at(&site, START)
                .map(|p| p.entity.id.clone())
    };
    let tenures = barred.tenures.iter().filter(|t| t.office_id == office);
    for tenure in tenures.filter(|t| bans(t.valid.start_year)) {
        assert_ne!(tenure.person_slot, Some(chosen), "{:?}", tenure.valid);
    }
}

#[test]
fn known_member_of_proscribed_group_cannot_hold_office() {
    let mut world = world();
    let site = world.site(0);
    let catalogue = world.canon.catalogue.clone();
    let open_group = world
        .region
        .groups
        .iter()
        .find(|g| {
            g.site_id == site
                && catalogue.archetype_alignment[&g.entity.archetype_id].default_visibility
                    == Visibility::Open
                && !catalogue.archetype_alignment[&g.entity.archetype_id].jurisdiction_organ
                && g.entity.existence.contains(1340)
        })
        .map(|g| g.entity.id.clone())
        .expect("an open group on the site");
    for standing in &mut world.region.standings {
        if standing.subject == ObjectRef::Entity(open_group.clone()) {
            standing.status = LegalStatus::Proscribed;
        }
    }
    let packet = packet(&world, &village(&site, 220));
    let index = packet
        .groups
        .iter()
        .position(|g| g.entity.id == open_group)
        .unwrap() as u32;
    let bound: BTreeSet<&str> = packet
        .offices
        .iter()
        .filter(|o| o.requirements.jurisdiction_bound)
        .map(|o| o.office_id.as_str())
        .collect();
    let members: Vec<_> = packet
        .memberships
        .iter()
        .filter(|m| m.group == index)
        .collect();
    assert!(!members.is_empty());
    let existence = packet.groups[index as usize].entity.existence;
    for tenure in packet
        .tenures
        .iter()
        .filter(|t| bound.contains(t.office_id.as_str()))
    {
        let start = tenure.valid.start_year;
        let already = members.iter().any(|m| {
            Some(m.person_slot) == tenure.person_slot
                && m.valid.start_year < start
                && start < m.valid.end_year
        });
        assert!(!(existence.contains(start) && already), "{start}");
    }
    assert!(packet
        .relations
        .iter()
        .any(|r| r.predicate == Predicate::InConflictWith
            && r.object == ObjectRef::Entity(open_group.clone())));
}

#[test]
fn same_firm_archetype_yields_several_conduct_classes() {
    let world = world();
    let packet = packet(&world, &village(&world.site(0), 10));
    let classes: BTreeSet<ConductClass> = packet
        .groups
        .iter()
        .filter(|g: &&GroupRecord| g.entity.archetype_id == "base_workshop")
        .map(|g| conduct_class(g.axes, &packet.organization_thresholds))
        .collect();
    assert!(classes.len() >= 3, "{classes:?}");
    assert!(classes.contains(&ConductClass::Ordinary));
    for group in &packet.groups {
        assert_eq!(
            group.name.tone,
            conduct_class(group.axes, &packet.organization_thresholds)
        );
        assert!(group.entity.name_tree.is_some());
        assert!(group.name_fallback.is_none());
        assert_eq!(group.name.provenance.rule_id, super::naming::NAMING_RULE);
    }
    let group = packet.groups.first().unwrap();
    let mut entity = group.entity.clone();
    let site = world
        .canon
        .atlas
        .settlements
        .iter()
        .find(|s| s.site_id == group.site_id)
        .unwrap();
    super::social::initialize_organization_name(
        &world.region.social,
        None,
        &mut entity,
        site,
        &world.region.region_id,
        super::model::AlignmentAxes::new(10_000, 10_000).unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(
        entity.name_tree, group.entity.name_tree,
        "later conduct never renames a group"
    );
}

#[test]
fn ancestry_never_sets_alignment() {
    let world = world();
    let site = world.site(0);
    let humans = village(&site, 40);
    let mut others = humans.clone();
    others.iter_mut().for_each(|p| p.ancestry = "elf-demo");
    let (a, b) = (packet(&world, &humans), packet(&world, &others));
    assert_eq!(a.people, b.people);
    assert_eq!(a.tenures, b.tenures);
}

#[test]
fn cohesion_can_cross_split_threshold() {
    let mut world = world();
    let site = world.site(0);
    let mut shock = world.region.shocks[0].clone();
    shock.site_id = site.clone();
    shock.valid = TimeSpan {
        start_year: 1310,
        end_year: 1345,
    };
    world.region.shocks = (0..3)
        .map(|i| {
            let mut shock = shock.clone();
            shock.source_event_id = format!("crisis-{i}");
            shock
        })
        .collect();
    let people = village(&site, 400);
    let packet = packet(&world, &people);
    let trigger = world.canon.catalogue.engine_defaults.split_trigger.clone();
    let crossed = packet
        .cohesion
        .iter()
        .filter(|c| i32::from(c.cohesion_bps) < trigger.cohesion_below_bps)
        .count();
    assert!(crossed > 0, "cohesion goes under the threshold");
    assert!(packet
        .cohesion
        .iter()
        .all(|c| c.source_event_ids.len() <= 3));
    let splits: Vec<_> = packet.events.iter().filter(|e| e.kind == "split").collect();
    assert!(!splits.is_empty(), "a sustained crisis splits a group");
    let children: Vec<&GroupRecord> = packet
        .groups
        .iter()
        .filter(|g| g.split_from.is_some())
        .collect();
    assert_eq!(children.len(), splits.len());
    for (index, child) in packet.groups.iter().enumerate() {
        if child.split_from.is_none() {
            continue;
        }
        let year = child.entity.existence.start_year;
        let moved = packet
            .memberships
            .iter()
            .filter(|m| m.group == index as u32 && m.valid.start_year == year)
            .count();
        assert!(moved > 0);
        // A transfer closes the old link: nobody counts in both groups.
        let source = packet
            .groups
            .iter()
            .position(|g| Some(&g.entity.id) == child.split_from.as_ref())
            .unwrap();
        let moved_out = packet
            .memberships
            .iter()
            .filter(|m| m.group == source as u32 && m.valid.end_year == year)
            .filter(|m| m.closed_by == Some(MembershipEnd::Split))
            .count();
        assert_eq!(moved, moved_out);
        assert!(packet
            .relations
            .iter()
            .any(|r| r.predicate == Predicate::SplitFrom
                && r.subject == ObjectRef::Entity(child.entity.id.clone())));
    }
}

#[test]
fn every_site_has_its_anchor_and_a_vacancy_is_a_valid_result() {
    let world = world();
    let packet = packet(&world, &[]);
    for site in &world.domain().site_ids {
        let office = mayor_office(&packet, site).unwrap();
        let view = resolve_office(&packet, &office.office_id, 1360).unwrap();
        assert_eq!(view.status, OfficeStatus::Vacant, "no person is invented");
        assert!(view.holder.is_none());
    }
    assert!(packet.groups.iter().all(|g| g.entity.name_tree.is_some()));
    assert!(packet.groups.iter().all(|g| g.name_fallback.is_none()));
    let mut missing = world.region.clone();
    std::sync::Arc::make_mut(&mut missing.social).naming_pack = None;
    // Simulate a region created without the pack as well as a packet without it.
    for group in &mut missing.groups {
        group.entity.name_tree = None;
    }
    let fallback = build_from_result(&missing, &table(&world, &[]), END).unwrap();
    assert!(fallback.groups.iter().all(|g| g.entity.name_tree.is_none()
        && g.name_fallback.as_ref().is_some_and(
            |name| name.provenance.status == super::model::ProvenanceStatus::Synthetic
        )));
    assert_eq!(packet.memberships, fallback.memberships);
    assert_eq!(packet.tenures, fallback.tenures);
    assert_eq!(packet.events, fallback.events);
}

#[test]
fn relation_id_is_independent_of_the_window() {
    let world = world();
    let site = world.site(0);
    let packet = packet(&world, &village(&site, 150));
    let span = TimeSpan {
        start_year: 1350,
        end_year: 1360,
    };
    let window = project_window(&packet, span);
    let full: BTreeMap<_, _> = packet
        .memberships
        .iter()
        .map(|m| ((m.group, m.person_slot, m.opened_year), m))
        .collect();
    let clipped: Vec<_> = window
        .memberships
        .iter()
        .filter(|m| m.opened_year < span.start_year)
        .collect();
    assert!(!clipped.is_empty(), "the window must cut some relations");
    for shown in clipped {
        assert_eq!(shown.valid.start_year, span.start_year);
        let original = full[&(shown.group, shown.person_slot, shown.opened_year)];
        let from_window = membership_relation(&packet, shown).unwrap();
        let from_full = membership_relation(&packet, original).unwrap();
        assert_eq!(from_window.id, from_full.id);
        assert_eq!(
            from_window.valid, shown.valid,
            "only the interval is clipped"
        );
        from_full.validate().unwrap();
    }
}

#[test]
fn closing_event_is_not_the_cause_of_a_relation() {
    let mut world = world();
    let site = world.site(0);
    let mut shock = world.region.shocks[0].clone();
    shock.site_id = site.clone();
    shock.valid = TimeSpan {
        start_year: 1310,
        end_year: 1345,
    };
    world.region.shocks = (0..3)
        .map(|i| {
            let mut shock = shock.clone();
            shock.source_event_id = format!("crisis-{i}");
            shock
        })
        .collect();
    let packet = packet(&world, &village(&site, 400));
    let year_of = |id: &str| packet.events.iter().find(|e| e.id == id).unwrap().year;
    let mut closed = 0;
    let mut opened = 0;
    for membership in &packet.memberships {
        let relation = membership_relation(&packet, membership).unwrap();
        if let Some(cause) = &relation.cause_event_id {
            assert_eq!(year_of(cause), membership.opened_year);
            opened += 1;
        }
        if let Some(end) = &relation.end_cause_event_id {
            assert_eq!(year_of(end), membership.valid.end_year);
            assert_ne!(relation.cause_event_id.as_ref(), Some(end));
            closed += 1;
        }
    }
    assert!(
        closed > 0 && opened > 0,
        "preemptions, then split transfers"
    );
}

/// Both charges end in 1305: the lower one must not hold its holder back from the
/// mayoralty, which is assigned first (§7.3).
#[test]
fn all_charges_close_before_any_is_assigned() {
    let world = world();
    let site = world.site(0);
    let people = [adult(1270, &site), adult(1271, &site)];
    let free = packet(&world, &people);
    let office = mayor_office(&free, &site).unwrap().office_id.clone();
    let mayor = holders(&free, &office)[0].2.unwrap();
    let other = 1 - mayor;
    let lower = free
        .tenures
        .iter()
        .find(|t| t.person_slot == Some(other) && t.valid.start_year == START)
        .unwrap();
    assert_eq!(
        lower.valid.end_year, 1305,
        "the other charge ends the same year"
    );
    let mut mortal = people.to_vec();
    mortal[mayor as usize].death = Some(1305);
    let packet = packet(&world, &mortal);
    let view = resolve_office(&packet, &office, 1305).unwrap();
    assert_eq!(view.status, OfficeStatus::Held, "no artificial vacancy");
    assert_eq!(view.tenure.unwrap().person_slot, Some(other));
    let selection = packet.events.iter().find(|e| e.kind == "office_selection");
    assert_eq!(selection.map(|e| e.phase as u8), Some(60));
}

#[test]
fn split_offices_are_filled_the_same_year() {
    let mut world = world();
    let site = world.site(0);
    let mut shock = world.region.shocks[0].clone();
    shock.site_id = site.clone();
    shock.valid = TimeSpan {
        start_year: 1310,
        end_year: 1345,
    };
    world.region.shocks = (0..3)
        .map(|i| {
            let mut shock = shock.clone();
            shock.source_event_id = format!("crisis-{i}");
            shock
        })
        .collect();
    let packet = packet(&world, &village(&site, 400));
    let mut checked = 0;
    for child in packet.groups.iter().filter(|g| g.split_from.is_some()) {
        for office in packet
            .offices
            .iter()
            .filter(|o| o.organization_id == child.entity.id)
        {
            let year = child.entity.existence.start_year;
            let first = packet
                .tenures
                .iter()
                .find(|t| t.office_id == office.office_id)
                .unwrap();
            assert_eq!(first.valid.start_year, year);
            checked += 1;
        }
    }
    assert!(checked > 0, "a split association has a head");
}

#[test]
fn holder_is_never_recruited_into_a_group_that_bars_his_charge() {
    let mut world = world();
    let site = world.site(0);
    let catalogue = world.canon.catalogue.clone();
    let open_group = world
        .region
        .groups
        .iter()
        .find(|g| {
            let traits = &catalogue.archetype_alignment[&g.entity.archetype_id];
            g.site_id == site
                && traits.default_visibility == Visibility::Open
                && !traits.jurisdiction_organ
                && g.entity.existence.contains(1340)
        })
        .map(|g| g.entity.id.clone())
        .expect("an open group on the site");
    let subject = ObjectRef::Entity(open_group.clone());
    let mut spans = Vec::new();
    for standing in &mut world.region.standings {
        if standing.subject == subject {
            standing.status = LegalStatus::Proscribed;
            spans.push((standing.jurisdiction.clone(), standing.valid));
        }
    }
    let packet = packet(&world, &village(&site, 220));
    let index = packet
        .groups
        .iter()
        .position(|g| g.entity.id == open_group)
        .unwrap() as u32;
    let bound: BTreeSet<&str> = packet
        .offices
        .iter()
        .filter(|o| o.requirements.jurisdiction_bound)
        .map(|o| o.office_id.as_str())
        .collect();
    let members: Vec<_> = packet
        .memberships
        .iter()
        .filter(|m| m.group == index)
        .collect();
    assert!(!members.is_empty());
    for tenure in packet
        .tenures
        .iter()
        .filter(|t| bound.contains(t.office_id.as_str()))
    {
        for year in tenure.valid.start_year..tenure.valid.end_year {
            let office = packet
                .offices
                .iter()
                .find(|o| o.office_id == tenure.office_id)
                .unwrap();
            let rules = world.canon.controller_at(&office.site_id, year).unwrap();
            let proscribes = spans.iter().any(|(j, span)| {
                *j == ObjectRef::Entity(rules.entity.id.clone()) && span.contains(year)
            });
            let both = members
                .iter()
                .any(|m| Some(m.person_slot) == tenure.person_slot && m.valid.contains(year));
            assert!(!(proscribes && both), "{year}");
        }
    }
}

#[test]
fn association_head_is_one_of_its_members() {
    let world = world();
    let site = world.site(0);
    let packet = packet(&world, &village(&site, 300));
    let mut heads = 0;
    for tenure in packet
        .tenures
        .iter()
        .filter(|t| t.mode == TenureMode::Effective)
    {
        let office = packet
            .offices
            .iter()
            .find(|o| o.office_id == tenure.office_id)
            .unwrap();
        if !office.requirements.member_only {
            continue;
        }
        let group = packet
            .groups
            .iter()
            .position(|g| g.entity.id == office.organization_id)
            .unwrap() as u32;
        for year in tenure.valid.start_year..tenure.valid.end_year {
            let member = packet.memberships.iter().any(|m| {
                m.group == group
                    && Some(m.person_slot) == tenure.person_slot
                    && m.valid.contains(year)
            });
            assert!(member, "{year}");
        }
        heads += 1;
    }
    assert!(heads > 0);
}

/// Elders found the groups and die in 1320; the young who replace them are under the
/// minimum age of a charge until 1326.
#[test]
fn dormant_association_goes_vacant_with_its_penalty() {
    let mut world = world();
    world.region.shocks.clear();
    let site = world.site(0);
    let elders = (0..40).map(|i| Person {
        death: Some(1320),
        family: i,
        ..adult(1270, &site)
    });
    let young = (0..60).map(|i| Person {
        family: 100 + i,
        ..adult(1301, &site)
    });
    let people: Vec<Person> = elders.chain(young).collect();
    let packet = packet(&world, &people);
    let vacant: Vec<_> = packet
        .tenures
        .iter()
        .filter(|t| t.mode == TenureMode::Vacant && t.valid.contains(1322))
        .filter_map(|t| packet.offices.iter().find(|o| o.office_id == t.office_id))
        .filter(|o| o.requirements.member_only)
        .collect();
    assert!(!vacant.is_empty(), "no member can hold the charge");
    let penalized = vacant.iter().any(|office| {
        let group = packet
            .groups
            .iter()
            .position(|g| g.entity.id == office.organization_id)
            .unwrap() as u32;
        packet
            .cohesion
            .iter()
            .any(|c| c.group == group && c.year == 1322 && c.vacancy_penalty && c.delta_bps == -400)
    });
    assert!(penalized);
}

#[test]
fn names_follow_their_keys_and_never_touch_facts() {
    use super::display_names::{person_names, place_labels, render_tree};
    let world = world();
    let site = world.site(0);
    let people = village(&site, 30);
    let packet = packet(&world, &people);
    let mut with_parent = table(&world, &people);
    with_parent.candidate_table.father_slot[1] = Some(0);
    with_parent.candidate_table.family_key[1] = "family/other".into();
    let names = person_names(
        world.canon.world_seed,
        &packet.domain_id,
        &with_parent.candidate_table,
        &packet.people,
    )
    .unwrap();
    let (father, son) = (names[0].as_ref().unwrap(), names[1].as_ref().unwrap());
    assert_eq!(father.surname, son.surname, "a surname runs down the line");
    assert_eq!(packet.person_names[3], names[3], "same key, same name");
    let labels = place_labels(&world.canon).unwrap();
    let texts: BTreeSet<_> = world
        .canon
        .atlas
        .settlements
        .iter()
        .map(|s| labels[&s.site_id].text.clone())
        .collect();
    assert_eq!(texts.len(), world.canon.atlas.settlements.len(), "distinct");
    assert!(texts.iter().all(|t| !t.contains('/')));
    let mut languages_differ = false;
    let mut slugs = 0;
    for group in &packet.groups {
        let tree = group.entity.name_tree.as_ref().unwrap();
        let rendered = render_tree(
            tree,
            &labels,
            &packet.family_surnames,
            world.canon.world_seed,
            &packet.domain_id,
        )
        .unwrap()
        .unwrap();
        languages_differ |= rendered["en"] != rendered["fr"];
        slugs += rendered.values().filter(|t| t.contains('/')).count();
    }
    assert!(languages_differ);
    assert_eq!(slugs, 0, "every ref is a real name");
}
