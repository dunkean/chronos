//! Invariants of the root chronology (Roadmap §9).

use std::collections::{BTreeMap, BTreeSet};

use chronos_core::*;

fn virtual_config(seed: u64) -> Config {
    Config {
        seed,
        map_mode: "virtual".into(),
        ..Config::default()
    }
}

/// Small hand-made fixture for tests only (synthetic, not real source data).
fn test_fixture() -> StackFixture {
    let (w, h) = (32, 24);
    let elevation_m = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                -50.0
            } else {
                100.0 + (x * 7 % 13) as f64 * 20.0
            }
        })
        .collect();
    let sites: Vec<BootstrapSite> = (0..14)
        .map(|i| BootstrapSite {
            id: format!("mu-{i:02}"),
            name: if i == 0 {
                Some("Capitale-Test".into())
            } else {
                None
            },
            x: 0.1 + 0.8 * ((i % 7) as f64 / 6.0),
            y: if i < 7 { 0.3 } else { 0.7 },
            region_id: if i % 7 < 4 {
                "west".into()
            } else {
                "east".into()
            },
            culture_id: "french".into(),
            kind: match i {
                0 | 13 => "city",
                3 | 10 => "town",
                _ => "village",
            }
            .into(),
            population: [
                9000, 800, 700, 3000, 600, 900, 650, 750, 820, 640, 2800, 700, 660, 8500,
            ][i],
            resources: Resources {
                food: 50.0,
                water: 60.0,
                timber: 40.0,
                iron: (i * 9 % 100) as f64,
                stone: 30.0,
                salt: 20.0,
            },
        })
        .collect();
    let mut routes = Vec::new();
    let mut link = |a: usize, b: usize| {
        let (sa, sb) = (&sites[a], &sites[b]);
        routes.push(Route {
            id: format!("r-{a}-{b}"),
            from: sa.id.clone(),
            to: sb.id.clone(),
            distance_km: 40.0,
            travel_days: 0.0,
            path: vec![vec![sa.x, sa.y], vec![sb.x, sb.y]],
            kind: "road".into(),
            source: "test".into(),
        });
    };
    for i in 0..6 {
        link(i, i + 1);
        link(i + 7, i + 8);
    }
    for i in 0..7 {
        link(i, i + 7);
    }
    // Three generations of families covering 1270–1430.
    let mut people = Vec::new();
    for f in 0..12 {
        let site = f % 14;
        let g0 = format!("ll-{f}-0");
        people.push(person(
            &g0,
            1270 + f as i32,
            Some(1335 + f as i32),
            None,
            site,
        ));
        for c in 0..3 {
            let g1 = format!("ll-{f}-1{c}");
            people.push(person(
                &g1,
                1300 + f as i32 + 3 * c,
                Some(1360 + 5 * c + f as i32),
                Some(&g0),
                site,
            ));
            for k in 0..2 {
                let g2 = format!("ll-{f}-2{c}{k}");
                let death = if k == 0 {
                    Some(1405 + c + f as i32)
                } else {
                    None
                };
                people.push(person(
                    &g2,
                    1340 + 2 * c + k + f as i32,
                    death,
                    Some(&g1),
                    (site + k as usize) % 14,
                ));
            }
        }
    }
    StackFixture {
        schema_version: 1,
        seed: 42,
        map: MapData {
            width: w,
            height: h,
            elevation_m,
            bounds: [0.0, 40.0, 8.0, 46.0],
            radius_km: 6371.0,
            source: "test".into(),
        },
        sites,
        routes,
        names: NamePools::default(),
        blueprints: vec![
            Blueprint {
                id: "po-kingdom".into(),
                label: "Kingdom".into(),
                kind: "monarchy".into(),
                office_title: "King".into(),
                succession: "hereditary primogeniture".into(),
                source: "test".into(),
                role_id: "king".into(),
                max_holders: Some(1),
                office_title_en: None,
                collective_office: None,
                roles: vec![BlueprintRole {
                    id: "king".into(),
                    collective: false,
                }],
            },
            Blueprint {
                // Collective office: the league is governed by its assembly.
                id: "po-city-league".into(),
                label: "City league".into(),
                kind: "merchant_league".into(),
                office_title: "Assemblée".into(),
                succession: "vote".into(),
                source: "test".into(),
                role_id: "assembly".into(),
                max_holders: None,
                office_title_en: None,
                collective_office: None,
                roles: vec![
                    BlueprintRole {
                        id: "assembly".into(),
                        collective: true,
                    },
                    BlueprintRole {
                        id: "delegate".into(),
                        collective: false,
                    },
                ],
            },
        ],
        people,
        sources: vec![Source {
            id: "test-source".into(),
            label: "Test".into(),
            status: "synthetic".into(),
            detail: "fixture de test".into(),
            fingerprint: "x".into(),
        }],
        genealogy_start_year: Some(1300),
        genealogy_end_year: Some(1430),
        people_window: None,
        warnings: Vec::new(),
    }
}

fn person(
    id: &str,
    birth: i32,
    death: Option<i32>,
    father: Option<&str>,
    site: usize,
) -> BootstrapPerson {
    BootstrapPerson {
        id: id.into(),
        birth_year: birth,
        death_year: death,
        father_id: father.map(Into::into),
        mother_id: None,
        site_index: site,
        sex: "M".into(),
        external_id: format!("ext-{id}"),
        name: None,
    }
}

fn stack_config() -> Config {
    Config {
        site_count: 14,
        population: 40_000,
        ..Config::default()
    }
}

#[test]
fn same_config_replays_byte_identical_export() {
    let cfg = virtual_config(7);
    let a = serde_json::to_string(&generate_world(&cfg, Err("absent")).unwrap()).unwrap();
    let b = serde_json::to_string(&generate_world(&cfg, Err("absent")).unwrap()).unwrap();
    assert_eq!(a, b);
    let other = generate_world(&virtual_config(8), Err("absent")).unwrap();
    assert_ne!(
        serde_json::from_str::<World>(&a).unwrap().canon_id,
        other.canon_id
    );
}

#[test]
fn generated_worlds_pass_validation_and_budgets() {
    let fx = test_fixture();
    let mut worlds = Vec::new();
    for seed in [1, 42, 99] {
        for mode in ["mixed", "council"] {
            worlds.push(
                generate_world(
                    &Config {
                        institution_mode: mode.into(),
                        ..virtual_config(seed)
                    },
                    Err("absent"),
                )
                .unwrap(),
            );
        }
        worlds.push(
            generate_world(
                &Config {
                    seed,
                    ..stack_config()
                },
                Ok(&fx),
            )
            .unwrap(),
        );
    }
    for w in &worlds {
        validate_world(w).unwrap_or_else(|e| panic!("seed {}: {e}", w.config.seed));
        for e in w
            .events
            .iter()
            .filter(|e| e.kind == "migration" || e.kind == "reconstruction")
        {
            assert_eq!(
                e.effects.iter().map(|f| f.population_delta).sum::<i64>(),
                0,
                "{} must conserve people",
                e.id
            );
        }
        // Each office has at most one holder at a time: successions follow each other.
        let mut holder: BTreeMap<&str, Option<&str>> = w
            .organizations
            .iter()
            .map(|o| (o.id.as_str(), o.holder_id.as_deref()))
            .collect();
        for e in w.events.iter().filter(|e| e.kind == "succession") {
            let org = e.organization_ids[0].as_str();
            assert_eq!(
                holder[org],
                e.person_ids.get(1).map(String::as_str),
                "{}",
                e.id
            );
            holder.insert(org, e.person_ids.first().map(String::as_str));
        }
        // A person is the father or the mother of their children, never both.
        let fathers: BTreeSet<_> = w
            .people
            .iter()
            .filter_map(|p| p.father_id.as_deref())
            .collect();
        let mothers: BTreeSet<_> = w
            .people
            .iter()
            .filter_map(|p| p.mother_id.as_deref())
            .collect();
        assert!(
            fathers.is_disjoint(&mothers),
            "{:?}",
            fathers.intersection(&mothers).collect::<Vec<_>>()
        );
    }
}

#[test]
fn free_scenario_with_zero_rates_has_no_forced_crises() {
    let calm = |c: Config| Config {
        scenario: "free".into(),
        war_rate: 0.0,
        epidemic_rate: 0.0,
        ..c
    };
    for w in [
        generate_world(&calm(virtual_config(42)), Err("absent")).unwrap(),
        generate_world(&calm(stack_config()), Ok(&test_fixture())).unwrap(),
    ] {
        validate_world(&w).unwrap();
        let forced: Vec<_> = w
            .events
            .iter()
            .filter(|e| ["war", "battle", "epidemic", "hero", "treaty"].contains(&e.kind.as_str()))
            .map(|e| &e.title)
            .collect();
        assert!(forced.is_empty(), "{forced:?}");
    }
    // The scenario is part of the canon identity.
    let free = generate_world(
        &Config {
            scenario: "free".into(),
            ..virtual_config(42)
        },
        Err("absent"),
    )
    .unwrap();
    assert_ne!(
        free.canon_id,
        generate_world(&virtual_config(42), Err("absent"))
            .unwrap()
            .canon_id
    );
}

#[test]
fn collective_office_has_no_personal_holder_or_succession() {
    let w = generate_world(&stack_config(), Ok(&test_fixture())).unwrap();
    let league = w
        .organizations
        .iter()
        .find(|o| o.kind == "merchant_league")
        .unwrap();
    assert!(league.collective_office && league.holder_id.is_none());
    assert!(!w
        .events
        .iter()
        .any(|e| e.kind == "succession" && e.organization_ids.contains(&league.id)));
    let crown = w
        .organizations
        .iter()
        .find(|o| o.blueprint_id == "po-kingdom")
        .unwrap();
    assert!(!crown.collective_office && crown.holder_id.is_some());
}

#[test]
fn pivot_epidemic_kills_exactly_floor_thirty_percent_in_1356() {
    for w in [
        generate_world(&virtual_config(42), Err("absent")).unwrap(),
        generate_world(&stack_config(), Ok(&test_fixture())).unwrap(),
    ] {
        let wave = w
            .events
            .iter()
            .find(|e| e.kind == "epidemic" && e.year == 1354 && e.end_year == 1360)
            .expect("pivot wave");
        let hit = w
            .events
            .iter()
            .find(|e| {
                e.year == 1356
                    && e.causes.contains(&wave.id)
                    && e.effects[0].summary.contains("× 3/10")
            })
            .expect("pivot city hit");
        let site = &hit.effects[0].site_id;
        let snap = w.snapshots.iter().find(|s| s.year == 1356).unwrap();
        let p1356 = snap
            .sites
            .iter()
            .find(|s| &s.site_id == site)
            .unwrap()
            .population;
        assert_eq!(hit.effects[0].population_delta, -((p1356 * 3 / 10) as i64));
    }
}

#[test]
fn pivot_war_destroys_exactly_three_villages_after_travel_time() {
    for w in [
        generate_world(&virtual_config(42), Err("absent")).unwrap(),
        generate_world(&stack_config(), Ok(&test_fixture())).unwrap(),
    ] {
        let war = w
            .events
            .iter()
            .find(|e| e.kind == "war" && e.description.contains("pivot"))
            .expect("pivot war");
        let sacks: Vec<_> = w
            .events
            .iter()
            .filter(|e| {
                e.causes.contains(&war.id)
                    && e.effects
                        .iter()
                        .any(|f| f.summary.contains("village détruit"))
            })
            .collect();
        assert_eq!(sacks.len(), 3, "{:?}", w.warnings);
        for e in &sacks {
            let site = w.sites.iter().find(|s| s.id == e.site_ids[0]).unwrap();
            assert!(site.kind == "village", "{} is {}", site.id, site.kind);
            assert!(e.year >= war.year && e.year <= war.end_year);
            // Remaining population after a sack is zero until reconstruction.
            let after = w.snapshots.iter().find(|s| s.year == e.year + 1).unwrap();
            assert_eq!(
                after
                    .sites
                    .iter()
                    .find(|s| s.site_id == site.id)
                    .unwrap()
                    .population,
                0
            );
        }
        // Armies never fight before they could arrive (days of march < 1 year here).
        for b in w.events.iter().filter(|e| e.kind == "battle") {
            let cause = w.events.iter().find(|e| e.id == b.causes[0]).unwrap();
            assert!(b.year >= cause.year && b.year <= cause.end_year);
        }
    }
}

#[test]
fn imported_people_keep_their_lives_and_heroes_are_alive_adults() {
    let fx = test_fixture();
    let w = generate_world(&stack_config(), Ok(&fx)).unwrap();
    let by_id: BTreeMap<_, _> = w.people.iter().map(|p| (p.id.as_str(), p)).collect();
    for r in &fx.people {
        let p = by_id[r.id.as_str()];
        assert_eq!(
            (p.birth_year, p.death_year, &p.father_id, &p.mother_id),
            (r.birth_year, r.death_year, &r.father_id, &r.mother_id)
        );
        assert_eq!(p.source, "lineage-lab");
        assert_eq!(p.external_id.as_deref(), Some(r.external_id.as_str()));
    }
    let hero = w
        .events
        .iter()
        .find(|e| e.kind == "hero" && e.importance >= 0.9)
        .expect("pivot hero");
    assert_eq!(hero.year, 1398);
    let p = by_id[hero.person_ids[0].as_str()];
    assert_eq!(p.source, "lineage-lab");
    assert!(
        p.father_id.is_some()
            && hero.year - p.birth_year >= 16
            && p.death_year.is_none_or(|d| hero.year < d)
    );
    assert!(w
        .sources
        .iter()
        .any(|s| s.id == "stack-fixture" && s.status == "imported"));
    assert_eq!(
        (w.genealogy_start_year, w.genealogy_end_year),
        (Some(1300), Some(1430))
    );

    // Before the archive starts, imported people take no role in events.
    let early = generate_world(
        &Config {
            start_year: 1250,
            ..stack_config()
        },
        Ok(&fx),
    )
    .unwrap();
    let imported: BTreeSet<_> = fx.people.iter().map(|p| p.id.as_str()).collect();
    for e in early.events.iter().filter(|e| e.year < 1300) {
        assert!(
            e.person_ids.iter().all(|p| !imported.contains(p.as_str())),
            "{}",
            e.id
        );
    }
    for o in &early.organizations {
        assert!(
            o.holder_id.as_deref().is_none_or(|h| !imported.contains(h)),
            "{}",
            o.id
        );
    }
}

#[test]
fn export_import_roundtrip_and_tampering_is_rejected() {
    let w = generate_world(&stack_config(), Ok(&test_fixture())).unwrap();
    let text = serde_json::to_string(&w).unwrap();
    let back: World = serde_json::from_str(&text).unwrap();
    assert_eq!(back, w);
    assert!(validate_world(&back).is_ok());

    let mut seed_changed = back.clone();
    seed_changed.config.seed += 1;
    assert!(validate_world(&seed_changed)
        .unwrap_err()
        .contains("empreinte"));

    let mut bad_cause = back.clone();
    let last = bad_cause.events.len() - 1;
    let late = bad_cause.events[last].id.clone();
    bad_cause.events[0].causes.push(late);
    assert!(validate_world(&bad_cause).is_err());
}

#[test]
fn only_closed_succession_identifiers_are_executed() {
    use chronos_core::social::Method;
    assert_eq!(
        Method::parse("Hereditary primogeniture"),
        Method::Hereditary
    );
    assert_eq!(Method::parse("member_vote"), Method::Election);
    assert_eq!(Method::parse("non-hereditary"), Method::Unsupported);
    assert_eq!(
        Method::parse("the eldest of the founders decides"),
        Method::Unsupported
    );
}

#[test]
fn missing_fixture_falls_back_without_claiming_imported_data() {
    let w = generate_world(&Config::default(), Err("fixtures/stack.json introuvable")).unwrap();
    assert!(w.sources.iter().all(|s| s.status != "imported"));
    assert!(w
        .sources
        .iter()
        .any(|s| s.id == "stack-fixture" && s.status == "unavailable"));
    assert!(w.people.iter().all(|p| p.source == "virtual"));
    assert!(w.warnings.iter().any(|m| m.contains("repli")));
}
