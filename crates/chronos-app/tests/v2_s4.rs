//! S4 exit proof on the real Lineage Lab worker: demo fixture, world seed 11, both
//! domains. Skips with a message when the interpreter is absent; set CHRONOS_PYTHON and
//! CHRONOS_LINEAGE_SRC to run it.
use chronos_app::v2::lineage_worker::WorkerSettings;
use chronos_app::v2::orchestrator::{Orchestrator, WorldInputs};
use chronos_app::v2::sha256_hex;
use chronos_app::v2::store::Store;
use chronos_core::v2::influence::influence_at;
use chronos_core::v2::model::{
    conduct_class, LegalStatus, ObjectRef, Predicate, TimeSpan, Visibility,
};
use chronos_core::v2::offices::{mayor_office, resolve_office, OfficeStatus, TenureMode};
use chronos_core::v2::social::{
    build_social_prefix, family_of, group_state, household_of, person_record, project_window,
    summarize,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn orchestrator() -> Option<Orchestrator> {
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lineage-cache");
    let settings = WorkerSettings::resolve(None, None, &root(), cache);
    if !settings.python.is_file() || !settings.lineage_src.join("lineage_lab").is_dir() {
        eprintln!("SKIPPED social packet: set CHRONOS_PYTHON and CHRONOS_LINEAGE_SRC");
        return None;
    }
    let root = root();
    let inputs = WorldInputs::load(
        &root.join("fixtures/social-v2/region-demo.world.json"),
        &root.join("fixtures/social-v2/region-demo.atlas.json"),
        &root.join("docs/design/chronos-social-v2/culture-bindings.v1.json"),
    )
    .unwrap();
    // Kept between runs: a warm store still goes through `validate_population`.
    let store = Store::new(Path::new(env!("CARGO_TARGET_TMPDIR")).join("s4-naming-store"));
    Some(Orchestrator::open(inputs, settings, store).unwrap())
}

#[test]
fn real_worker_packet_gives_a_stable_mayor_and_a_closed_balance() {
    let Some(orchestrator) = orchestrator() else {
        return;
    };
    let domains = orchestrator.domain_ids();
    assert_eq!(domains.len(), 2, "the demo has two domains");
    // Domain B before domain A, in a fresh orchestrator on the same store.
    let reverse = self::orchestrator().unwrap();
    let mut mayors: BTreeMap<String, Vec<(i32, Option<String>)>> = BTreeMap::new();
    for domain in &domains {
        let population = orchestrator.population(domain).unwrap();
        let region = orchestrator.region_of(domain).unwrap();
        let packet = orchestrator.social_packet(domain).unwrap();
        assert!(!packet.person_group_names.is_empty());
        assert!(packet
            .person_group_names
            .iter()
            .all(|n| n.name_tree.is_some() && n.name_fallback.is_none()));
        assert!(packet
            .person_group_names
            .iter()
            .any(
                |n| n.name_tree.as_ref().unwrap().value()["parts"]["owner"]["id"] == n.surname_ref
            ));
        let again = reverse.social_packet(&domains[1]).unwrap();
        let other = reverse.social_packet(&domains[0]).unwrap();
        let same = if *domain == domains[0] { other } else { again };
        assert_eq!(
            sha256_hex(&packet.canonical_bytes().unwrap()),
            sha256_hex(&same.canonical_bytes().unwrap()),
            "query order changed a packet"
        );
        assert_eq!(packet.balance.len(), 100);
        let table = &population.result().candidate_table;
        for row in &packet.balance {
            let census: u64 = population
                .result()
                .annual_censuses
                .iter()
                .filter(|c| c.year == row.year)
                .map(|c| c.population)
                .sum();
            assert_eq!(row.population, census);
            assert!(row.persons_with_affiliation <= row.population);
        }
        let classes: BTreeSet<_> = packet
            .groups
            .iter()
            .map(|g| g.entity.kind.as_str())
            .collect();
        for class in [
            "activity_unit",
            "association",
            "community",
            "friend_circle",
            "institution",
        ] {
            assert!(classes.contains(class), "{class}");
        }
        // No role outside life or residence; singleton offices never overlap.
        let mut held: BTreeMap<&str, Vec<TimeSpan>> = BTreeMap::new();
        for tenure in &packet.tenures {
            held.entry(&tenure.office_id)
                .or_default()
                .push(tenure.valid);
            if let (TenureMode::Effective, Some(slot)) = (tenure.mode, tenure.person_slot) {
                let row = slot as usize;
                assert!(table.birth_year[row] <= tenure.valid.start_year);
                assert!(table.death_year[row].is_none_or(|d| tenure.valid.end_year <= d));
                let office = packet
                    .offices
                    .iter()
                    .find(|o| o.office_id == tenure.office_id)
                    .unwrap();
                // The head of an association may live anywhere within its recruitment reach.
                let years = if office.requirements.member_only {
                    vec![]
                } else {
                    vec![tenure.valid.start_year, tenure.valid.end_year - 1]
                };
                for year in years {
                    let resident = population.residences_of(slot).iter().any(|r| {
                        r.site_id == office.site_id
                            && r.valid.start_year <= year
                            && (year < r.valid.end_year)
                    });
                    assert!(resident, "{year}");
                }
            }
        }
        for spans in held.values() {
            for pair in spans.windows(2) {
                assert_eq!(pair[0].end_year, pair[1].start_year, "gap or overlap");
            }
        }
        // Open proscribed groups: their known members stay out of the jurisdiction's
        // offices, and the conflict is a dated relation.
        let proscribed: BTreeSet<_> = packet
            .standings
            .iter()
            .filter(|s| s.status == LegalStatus::Proscribed)
            .filter_map(|s| match &s.subject {
                ObjectRef::Entity(id) => Some(id.as_str()),
                _ => None,
            })
            .collect();
        let conflicts = packet
            .groups
            .iter()
            .filter(|g| {
                g.visibility == Visibility::Open && proscribed.contains(g.entity.id.as_str())
            })
            .count();
        let at_war = packet
            .relations
            .iter()
            .filter(|r| r.predicate == Predicate::InConflictWith)
            .count();
        assert!(conflicts == 0 || at_war > 0);
        for standing in packet
            .standings
            .iter()
            .filter(|s| s.status == LegalStatus::Proscribed)
        {
            let (ObjectRef::Entity(subject), ObjectRef::Entity(jurisdiction)) =
                (&standing.subject, &standing.jurisdiction)
            else {
                continue;
            };
            let Some(group) = packet
                .groups
                .iter()
                .position(|g| &g.entity.id == subject && g.visibility == Visibility::Open)
            else {
                continue;
            };
            let members: Vec<_> = packet
                .memberships
                .iter()
                .filter(|m| m.group == group as u32)
                .collect();
            for tenure in &packet.tenures {
                let start = tenure.valid.start_year;
                let office = packet
                    .offices
                    .iter()
                    .find(|o| o.office_id == tenure.office_id)
                    .unwrap();
                let rules = orchestrator
                    .canon()
                    .controller_at(&office.site_id, start)
                    .is_some_and(|p| &p.entity.id == jurisdiction);
                if office.requirements.jurisdiction_bound && rules && standing.valid.contains(start)
                {
                    assert!(!members.iter().any(|m| {
                        Some(m.person_slot) == tenure.person_slot
                            && m.valid.start_year < start
                            && start < m.valid.end_year
                    }));
                }
            }
        }

        let (site_ids, years) = (
            packet.site_views.keys().cloned().collect::<Vec<_>>(),
            [1340, 1360, 1380],
        );
        for site in &site_ids {
            let office = mayor_office(&packet, site).unwrap();
            let forward: Vec<_> = years
                .iter()
                .map(|y| resolve_office(&packet, &office.office_id, *y).unwrap())
                .collect();
            let backward: Vec<_> = years
                .iter()
                .rev()
                .map(|y| resolve_office(&packet, &office.office_id, *y).unwrap())
                .collect();
            assert!(forward.iter().eq(backward.iter().rev()));
            for view in &forward {
                let holder = view.holder.as_ref().map(|h| h.person_id.clone());
                mayors
                    .entry(site.clone())
                    .or_default()
                    .push((view.at_year, holder));
                assert!(matches!(
                    view.status,
                    OfficeStatus::Held | OfficeStatus::Vacant
                ));
            }
        }
        let thresholds = &packet.organization_thresholds;
        let classes: BTreeSet<_> = packet
            .groups
            .iter()
            .filter(|g| g.entity.kind == "activity_unit")
            .map(|g| conduct_class(g.axes, thresholds))
            .collect();
        assert!(classes.len() >= 3);

        // A person of 1360 reads as household, family and overlapping affiliations.
        let view = resolve_office(
            &packet,
            &mayor_office(&packet, &site_ids[0]).unwrap().office_id,
            1360,
        )
        .unwrap();
        if let Some(holder) = view.holder {
            assert!(household_of(&population, holder.person_slot, 1360).is_some());
            let family = family_of(&population, holder.person_slot, 1360);
            let tags = BTreeSet::new();
            person_record(&packet, &population, holder.person_slot)
                .unwrap()
                .validate(&tags)
                .unwrap();
            assert!(
                family.lineage_members.is_empty()
                    || holder.source == chronos_core::v2::population::SlotSource::Simulated
            );
        }
        let group = packet
            .groups
            .iter()
            .position(|g| g.entity.existence.contains(1360))
            .unwrap();
        let state = group_state(&packet, group as u32, 1360).unwrap();
        assert_eq!(state.at_year, 1360);
        assert!(
            !influence_at(&region, &packet, &site_ids[0], "economy", 1360)
                .unwrap()
                .is_empty()
        );

        // Prefix of the same plan against the projection of the full packet (§9.5).
        let span = TimeSpan {
            start_year: 1300,
            end_year: 1351,
        };
        let prefix = build_social_prefix(&region, &population, span.end_year).unwrap();
        assert_eq!(
            project_window(&packet, span).tenures,
            project_window(&prefix, span).tenures
        );

        let summary = summarize(&packet, &population, &years, &site_ids).unwrap();
        println!("{}", serde_json::to_string(&summary.years).unwrap());
    }
    assert_eq!(mayors.len(), 20, "one mayor per site of the demo");
}
