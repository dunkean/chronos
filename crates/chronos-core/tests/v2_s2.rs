//! S2 exit proof: two domains over one century, seed 11, 1300-1400,
//! `chronos-social-2`, on the synthetic fixture `fixtures/social-v2/region-demo`.
use chronos_core::v2::atlas::{HistoryAtlas, SettlementAnchor};
use chronos_core::v2::catalog::HistoryCatalogue;
use chronos_core::v2::general::{build_canon, AuthorConstraints, GeneralCanon, WorldConfig};
use chronos_core::v2::ids::{Owner, SemanticAddress};
use chronos_core::v2::model::{
    ErrorCode, EventPhase, HistoryEvent, LegalStanding, LegalStatus, ObjectRef, ProvenanceStatus,
    TimeSpan,
};
use chronos_core::v2::population::{plan_canonical_bytes, plan_population, PopulationPlan};
use chronos_core::v2::region::{plan_region, RegionPlan};
use chronos_core::v2::validate::validate_legal_standings;
use std::collections::{BTreeMap, BTreeSet};

const REGION: &str = "region/demo";

fn atlas() -> HistoryAtlas {
    serde_json::from_str(include_str!(
        "../../../fixtures/social-v2/region-demo.atlas.json"
    ))
    .unwrap()
}
fn config() -> WorldConfig {
    serde_json::from_str(include_str!(
        "../../../fixtures/social-v2/region-demo.world.json"
    ))
    .unwrap()
}
fn catalogue() -> HistoryCatalogue {
    serde_json::from_str(include_str!(
        "../../../docs/design/chronos-social-v2/culture-bindings.v1.json"
    ))
    .unwrap()
}
fn canon_of(config: &WorldConfig, atlas: &HistoryAtlas) -> GeneralCanon {
    build_canon(config, atlas, &catalogue(), &AuthorConstraints::default()).unwrap()
}
fn canon() -> GeneralCanon {
    canon_of(&config(), &atlas())
}
fn plans(canon: &GeneralCanon, region: &RegionPlan) -> Vec<PopulationPlan> {
    canon
        .domains
        .iter()
        .filter(|d| d.region_id == region.region_id)
        .map(|d| plan_population(canon, region, &d.domain_id).unwrap())
        .collect()
}
fn bytes(plan: &PopulationPlan) -> Vec<u8> {
    plan_canonical_bytes(plan).unwrap()
}
/// Same manifest with crises switched off: only shocks differ.
fn without_crises() -> GeneralCanon {
    let mut quiet = config();
    quiet.processes.war_bps = 0;
    quiet.processes.epidemic_bps = 0;
    canon_of(&quiet, &atlas())
}
/// A second region, copied under other IDs: a wider world in the same manifest.
fn wider_atlas() -> HistoryAtlas {
    let east = |id: &str| {
        id.replace("region/demo", "region/east")
            .replace("/demo/", "/east/")
    };
    let base = atlas();
    let mut wide = base.clone();
    wide.region_ids.push(east(REGION));
    for cell in &base.cells {
        let mut copy = cell.clone();
        copy.cell_id = east(&cell.cell_id);
        copy.parent_id = cell.parent_id.as_deref().map(east);
        copy.neighbors = cell.neighbors.iter().map(|n| east(n)).collect();
        let ids = &mut copy.settlement_summary.site_ids;
        *ids = ids.iter().map(|s| east(s)).collect();
        wide.cells.push(copy);
    }
    wide.settlements
        .extend(base.settlements.iter().map(|s| SettlementAnchor {
            site_id: east(&s.site_id),
            cell_id: east(&s.cell_id),
            ..s.clone()
        }));
    for edge in &base.mobility_edges {
        let mut copy = edge.clone();
        copy.edge_id = east(&edge.edge_id);
        copy.from_site_id = east(&edge.from_site_id);
        copy.to_site_id = east(&edge.to_site_id);
        wide.mobility_edges.push(copy);
    }
    wide
}

#[test]
fn two_disjoint_domains_replay_byte_for_byte_over_a_century() {
    let (first, second) = (canon(), canon());
    assert_eq!(
        first.canonical_bytes().unwrap(),
        second.canonical_bytes().unwrap()
    );
    let (a, b) = (
        plan_region(&first, REGION).unwrap(),
        plan_region(&second, REGION).unwrap(),
    );
    assert_eq!(a.canonical_bytes().unwrap(), b.canonical_bytes().unwrap());
    let (plans_a, plans_b) = (plans(&first, &a), plans(&second, &b));
    assert_eq!(plans_a.len(), 2);
    assert_eq!(plans_a, plans_b);
    let sites: Vec<BTreeSet<_>> = first
        .domains
        .iter()
        .map(|d| d.site_ids.iter().collect())
        .collect();
    assert!(sites[0].is_disjoint(&sites[1]));
    assert_eq!(sites[0].len() + sites[1].len(), atlas().settlements.len());
    assert_eq!(
        first.events[0].provenance.status,
        ProvenanceStatus::Synthetic
    );
}
#[test]
fn opening_one_domain_does_not_alter_the_other() {
    let canon = canon();
    let region = plan_region(&canon, REGION).unwrap();
    let [a, b] = [&canon.domains[0], &canon.domains[1]].map(|d| d.domain_id.clone());
    let alone = plan_population(&canon, &region, &b).unwrap();
    plan_population(&canon, &region, &a).unwrap();
    let after = plan_population(&canon, &region, &b).unwrap();
    assert_eq!(bytes(&alone), bytes(&after));
}
#[test]
fn world_extent_does_not_reseed_domains() {
    let small = canon();
    let wide = canon_of(&config(), &wider_atlas());
    assert_eq!(wide.domains.len(), 4);
    let small_region = plan_region(&small, REGION).unwrap();
    let wide_region = plan_region(&wide, REGION).unwrap();
    assert_eq!(
        small_region.canonical_bytes().unwrap(),
        wide_region.canonical_bytes().unwrap()
    );
    let (a, b) = (plans(&small, &small_region), plans(&wide, &wide_region));
    assert_eq!(a.len(), 2);
    for (small_plan, wide_plan) in a.iter().zip(&b) {
        assert_eq!(small_plan.seed, wide_plan.seed);
        assert_eq!(small_plan.domain_id, wide_plan.domain_id);
        assert_eq!(bytes(small_plan), bytes(wide_plan));
    }
}
#[test]
fn plan_region_call_order_changes_nothing() {
    let canon = canon_of(&config(), &wider_atlas());
    let (first, second) = (REGION, "region/east");
    let forward = [
        plan_region(&canon, first).unwrap(),
        plan_region(&canon, second).unwrap(),
    ];
    let second_first = plan_region(&canon, second).unwrap();
    let first_last = plan_region(&canon, first).unwrap();
    assert_eq!(
        forward[0].canonical_bytes().unwrap(),
        first_last.canonical_bytes().unwrap()
    );
    assert_eq!(
        forward[1].canonical_bytes().unwrap(),
        second_first.canonical_bytes().unwrap()
    );
}
#[test]
fn generated_baseline_nodes_are_independent_of_shocks() {
    let crises = canon();
    let quiet = without_crises();
    let with_shocks = plan_region(&crises, REGION).unwrap();
    let no_shocks = plan_region(&quiet, REGION).unwrap();
    assert!(!with_shocks.shocks.is_empty());
    assert!(no_shocks.shocks.is_empty());
    assert_eq!(with_shocks.site_baselines, no_shocks.site_baselines);
    let (a, b) = (plans(&crises, &with_shocks), plans(&quiet, &no_shocks));
    for (shocked, plain) in a.iter().zip(&b) {
        assert_eq!(shocked.baseline_markers, plain.baseline_markers);
    }
}
#[test]
fn spared_village_keeps_baseline_target() {
    let canon = canon();
    let region = plan_region(&canon, REGION).unwrap();
    let plain = plan_region(&without_crises(), REGION).unwrap();
    let hit: BTreeSet<_> = region.shocks.iter().map(|s| s.site_id.as_str()).collect();
    let moved: BTreeSet<_> = region
        .displacements
        .iter()
        .flat_map(|d| [Some(&d.origin_site_id), d.destination_site_id.as_ref()])
        .flatten()
        .map(String::as_str)
        .collect();
    let ported: BTreeSet<_> = region
        .contexts
        .values()
        .flat_map(|c| &c.ports)
        .map(|p| p.entry_site_id.as_str())
        .collect();
    let spared: Vec<_> = region
        .site_baselines
        .keys()
        .filter(|s| {
            ![&hit, &moved, &ported]
                .iter()
                .any(|set| set.contains(s.as_str()))
        })
        .collect();
    assert!(!spared.is_empty());
    for site in spared {
        assert_eq!(region.site_baselines[site], plain.site_baselines[site]);
        // No regional average leaks in: its forecast is its own baseline.
        assert_eq!(
            region.population_expected_after_impacts[site],
            region.site_baselines[site]
        );
    }
    let plans = plans(&canon, &region);
    for site in hit.iter().copied() {
        let forecast = &region.population_expected_after_impacts[site];
        assert!(forecast
            .iter()
            .any(|(y, p)| *p != region.site_baselines[site][y]));
    }
    // The plan lists a shock only on its exposed sites.
    for plan in &plans {
        for shock in &plan.shocks {
            assert!(shock.site_ids.iter().all(|s| hit.contains(s.as_str())));
        }
    }
}
#[test]
fn local_shocks_follow_their_frame_and_travel_time() {
    let canon = canon();
    let region = plan_region(&canon, REGION).unwrap();
    let events: BTreeMap<_, _> = canon.events.iter().map(|e| (e.id.as_str(), e)).collect();
    assert!(canon.events.iter().any(|e| e.kind == "war"));
    assert!(canon.events.iter().any(|e| e.kind == "epidemic"));
    for shock in &region.shocks {
        let frame = events[shock.source_event_id.as_str()];
        assert!(shock.valid.start_year >= frame.year);
        assert!(shock.valid.start_year < frame.end_year.unwrap());
        if frame.kind == "war" {
            assert!(shock.valid.start_year > frame.year);
            assert_eq!(shock.severity_profile_id, "war.ruin");
        }
    }
    let ids: BTreeSet<_> = region.shocks.iter().map(|s| &s.episode_id).collect();
    assert_eq!(ids.len(), region.shocks.len());
}
fn standings_of<'a>(region: &'a RegionPlan, id: &str) -> Vec<&'a LegalStanding> {
    region
        .standings
        .iter()
        .filter(|s| s.subject == ObjectRef::Entity(id.into()))
        .collect()
}
#[test]
fn legal_chains_cover_status_changes_and_control_changes() {
    let canon = canon();
    let region = plan_region(&canon, REGION).unwrap();
    let events: BTreeMap<_, _> = canon
        .events
        .iter()
        .chain(&region.events)
        .map(|e| (e.id.as_str(), e))
        .collect();
    let kinds: BTreeSet<_> = events.values().map(|e| e.kind.as_str()).collect();
    for kind in [
        "edict",
        "charter",
        "heresy_trial",
        "banishment",
        "foundation",
        "dissolution",
        "treaty",
    ] {
        assert!(kinds.contains(kind), "{kind} absent from the demo century");
    }
    let statuses: BTreeSet<_> = region.standings.iter().map(|s| s.status).collect();
    assert!(statuses.contains(&LegalStatus::Proscribed));
    assert!(statuses.contains(&LegalStatus::Tolerated));
    // A late group opens its chain on its own foundation.
    let late = region
        .groups
        .iter()
        .find(|g| g.entity.existence.start_year > canon.timeline.start_year)
        .unwrap();
    let first = standings_of(&region, &late.entity.id)[0];
    let cause = first.cause_event_id.as_deref().unwrap();
    assert_eq!(events[cause].kind, "foundation");
    // A site that changed hands re-evaluates its groups under a charter.
    let moved = canon
        .site_control
        .iter()
        .find(|(_, episodes)| episodes.len() > 1)
        .expect("a control change in the demo century");
    let change = moved.1[1].valid.start_year;
    let charters: Vec<_> = events
        .values()
        .filter(|e| e.kind == "charter" && e.year == change && !e.causes.is_empty())
        .collect();
    assert!(!charters.is_empty());
    assert!(charters
        .iter()
        .all(|e| events[e.causes[0].as_str()].kind == "treaty"));
}
#[test]
fn control_relations_cite_the_treaty() {
    let canon = canon();
    for episodes in canon.site_control.values() {
        assert_eq!(episodes[0].valid.start_year, canon.timeline.start_year);
        assert!(episodes[0].cause_event_id.is_none());
        for pair in episodes.windows(2) {
            assert_eq!(pair[0].valid.end_year, pair[1].valid.start_year);
            let cause = pair[1].cause_event_id.as_ref().unwrap();
            let treaty = canon.events.iter().find(|e| &e.id == cause).unwrap();
            assert_eq!(treaty.kind, "treaty");
            assert_eq!(treaty.year, pair[1].valid.start_year);
        }
    }
}
fn life_event(group: &ObjectRef, kind: &str, year: i32) -> HistoryEvent {
    let owner = Owner::cell("w", "region");
    let address = SemanticAddress::event(&owner, kind, 1350, 0).unwrap();
    HistoryEvent {
        id: address.encode(),
        owner_scope: owner.scope().clone(),
        kind: kind.into(),
        year,
        end_year: None,
        phase: EventPhase::InstitutionsAndVacancies,
        causes: vec![],
        participants: vec![group.clone()],
        provenance: canon_free_provenance(),
    }
}
fn canon_free_provenance() -> chronos_core::v2::model::Provenance {
    chronos_core::v2::model::Provenance {
        status: ProvenanceStatus::Synthetic,
        source_ref: "s2-test".into(),
        rule_id: "s2-test-1".into(),
    }
}
#[test]
fn foundation_opens_only_its_own_subject_first_standing() {
    let owner = Owner::cell("w", "region");
    let entity = |slot: &str| {
        ObjectRef::Entity(
            SemanticAddress::entity(&owner, "guild", 1300, slot)
                .unwrap()
                .encode(),
        )
    };
    let (group, other, jurisdiction) = (entity("g"), entity("o"), entity("j"));
    let standing = |subject: &ObjectRef, cause: &HistoryEvent| LegalStanding {
        id: SemanticAddress::standing(&owner, subject, &jurisdiction, 1355, 0)
            .unwrap()
            .encode(),
        subject: subject.clone(),
        jurisdiction: jurisdiction.clone(),
        status: LegalStatus::Proscribed,
        valid: TimeSpan {
            start_year: 1355,
            end_year: 1400,
        },
        cause_event_id: Some(cause.id.clone()),
        end_cause_event_id: None,
        provenance: canon_free_provenance(),
        rule_id: "s2-test-1".into(),
    };
    let founding = life_event(&group, "foundation", 1355);
    let check = |s: LegalStanding| {
        validate_legal_standings(&[s], std::slice::from_ref(&founding), 1300, 1400)
    };
    assert!(check(standing(&group, &founding)).is_ok());
    assert_eq!(
        check(standing(&other, &founding)).unwrap_err().code,
        ErrorCode::InvalidLegalStanding
    );
}
#[test]
fn explicit_author_series_survive_unchanged() {
    let atlas = atlas();
    let series: BTreeMap<i32, u64> = chronos_core::v2::general::marker_dates(config().timeline)
        .into_iter()
        .map(|year| (year, 100))
        .collect();
    let constraints = AuthorConstraints {
        site_baselines: atlas
            .settlements
            .iter()
            .map(|s| (s.site_id.clone(), series.clone()))
            .collect(),
    };
    let canon = build_canon(&config(), &atlas, &catalogue(), &constraints).unwrap();
    let region = plan_region(&canon, REGION).unwrap();
    assert!(region.site_baselines.values().all(|s| s == &series));
    let mut partial = constraints.clone();
    partial.site_baselines.pop_first();
    assert!(build_canon(&config(), &atlas, &catalogue(), &partial).is_err());
}
