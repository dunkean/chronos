//! Régressions S0 : seed 42, chronos-social-2, schéma de plan 1.
use chronos_core::v2::{
    atlas::*, canonical::canonical_bytes, ids::*, model::*, population::*, region::*, validate::*,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const PERIOD: TimeSpan = TimeSpan {
    start_year: 1300,
    end_year: 1400,
};
fn provenance() -> Provenance {
    Provenance {
        status: ProvenanceStatus::Synthetic,
        source_ref: "fixture-s0".into(),
        rule_id: "s0-test-1".into(),
    }
}
fn event(id: &str, year: i32, phase: EventPhase, causes: &[&str]) -> HistoryEvent {
    HistoryEvent {
        id: id.into(),
        owner_scope: "region".into(),
        kind: "test".into(),
        year,
        phase,
        causes: causes.iter().map(|s| (*s).into()).collect(),
        participants: vec![],
        provenance: provenance(),
    }
}
fn fixture_atlas() -> HistoryAtlas {
    let settlements: Vec<_> = (0..12)
        .map(|i| SettlementAnchor {
            site_id: format!("site/{i:02}"),
            cell_id: "region".into(),
            coordinate_frame: "local-mm".into(),
            x_mm: i,
            y_mm: 0,
            foundation_year: 1250,
            initial_population: 100,
            population_expected_max: 200,
            occupation_episodes: vec![OccupationEpisode {
                valid: PERIOD,
                state: OccupationState::Active,
                capacity: 200,
                culture_weights: BTreeMap::from([("culture".into(), 10_000)]),
                activity_weights: BTreeMap::from([("agriculture".into(), 100)]),
            }],
        })
        .collect();
    HistoryAtlas {
        atlas_id: "atlas-s0".into(),
        source_fingerprint: "fixture-s0-1".into(),
        timeline: PERIOD,
        cells: vec![AtlasCell {
            cell_id: "region".into(),
            parent_id: None,
            neighbors: vec![],
            area_m2: 1000,
            bounds: MetricBounds {
                min_x_mm: 0,
                min_y_mm: 0,
                max_x_mm: 12,
                max_y_mm: 10,
            },
            coordinate_frame: "local-mm".into(),
            settlement_summary: SettlementSummary {
                site_ids: settlements.iter().map(|s| s.site_id.clone()).collect(),
                population_expected_max: 2400,
            },
        }],
        region_ids: vec!["region".into()],
        settlements,
        mobility_edges: vec![],
        external_contacts: vec![],
    }
}
fn contract(mode: ImpactMode) -> ImpactContract {
    ImpactContract {
        id: "impact/test/0".into(),
        parent_event_id: SemanticAddress::event("canon-s0-seed42", "region", "war", 1350, 0)
            .unwrap()
            .encode(),
        owner_scope: "region".into(),
        valid: PERIOD,
        target: ImpactTarget::Sites,
        selector: TargetSelector::All,
        eligibility_time: 1354,
        mode,
        exposure_model: ExposureModel::IndependentTargetV1,
        severity_profile: "war.medium".into(),
        timing_profile: "annual-1".into(),
        channel: "destruction".into(),
        cause: "war".into(),
    }
}
fn example_value() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/design/chronos-social-v2/village-population-plan.example.json"
    ))
    .unwrap()
}
fn example() -> PopulationPlan {
    serde_json::from_value(example_value()).unwrap()
}
fn references(p: &PopulationPlan) -> PlanReferences {
    PlanReferences {
        catalogue_fingerprints: p.catalogue_fingerprints.clone(),
        event_years: BTreeMap::from([
            ("event/demo/plague".into(), 1354),
            ("event/demo/displacement".into(), 1365),
        ]),
        office_ids: BTreeSet::from(["office/demo/mayor-a".into()]),
        history_cultures: p
            .settlements
            .iter()
            .map(|s| {
                (
                    s.history_culture_id.clone(),
                    p.demography_society_id.clone(),
                )
            })
            .collect(),
        culture_ids: BTreeSet::from(["european-organic".into(), "bastide".into()]),
        activity_ids: ["agriculture", "craft", "trade", "services"]
            .into_iter()
            .map(String::from)
            .collect(),
        contacts: p
            .boundary_ports
            .iter()
            .map(|port| {
                (
                    port.contact_id.clone(),
                    ExternalContact {
                        contact_id: port.contact_id.clone(),
                        owner_scope: "region/demo".into(),
                        entry_site_id: port.entry_site_id.clone(),
                        external_context_id: port.external_context_id.clone(),
                        external_profile_id: port.external_profile_id.clone(),
                        population_expected: port.population_expected,
                        valid: port.valid,
                        access_edge_id: None,
                    },
                )
            })
            .collect(),
        site_population_expected_max: BTreeMap::from([
            ("site/demo/a".into(), 1200),
            ("site/demo/b".into(), 500),
        ]),
    }
}

#[test]
fn address_roundtrip_preserves_every_field_and_type() {
    for (kind, fields) in [
        (
            AddressKind::Entity,
            vec!["w", "scope", "office", "-10", "mayor"],
        ),
        (
            AddressKind::Person,
            vec!["w", "domain", "algorithm-1", "17"],
        ),
        (AddressKind::Tenure, vec!["w", "office", "0"]),
        (
            AddressKind::Event,
            vec!["w", "scope", "plague", "1350", "2"],
        ),
        (
            AddressKind::Group,
            vec!["w", "scope", "group_slot", "farm", "1350", "2"],
        ),
        (
            AddressKind::Household,
            vec!["w", "domain", "household", "singleton/17", "1354"],
        ),
        (
            AddressKind::Transfer,
            vec!["w", "scope", "transfer", "a", "b", "1354", "2"],
        ),
        (AddressKind::Plan, vec!["w", "domain"]),
        (AddressKind::Domain, vec!["w", "region", "society", "0"]),
    ] {
        let address =
            SemanticAddress::from_fields(kind, fields.into_iter().map(String::from).collect())
                .unwrap();
        assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
        assert!(!address.encode().contains('='));
    }
    let a = SemanticAddress::person("world", "domain", "algo", "1").unwrap();
    let b = SemanticAddress::person("world", "domain", "algo", "2").unwrap();
    let relation = SemanticAddress::relation("scope", "parent_of", &a, &b, -5, 0).unwrap();
    assert_eq!(
        SemanticAddress::decode(&relation.encode()).unwrap(),
        relation
    );
    assert!(!relation.fields().contains(&a.encode()));
    assert_ne!(a.encode(), b.encode());
}
#[test]
fn address_longer_than_4096_characters_is_rejected() {
    assert!(SemanticAddress::person("w", "d", "a", &"x".repeat(4096)).is_err());
    assert!(SemanticAddress::decode(&format!("person:{}", "A".repeat(4096))).is_err());
}
#[test]
fn canonical_json_matches_shared_python_vectors() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../docs/design/chronos-social-v2/vectors/canonical-json.v1.json"
    ))
    .unwrap();
    for v in vectors["vectors"].as_array().unwrap() {
        let bytes = canonical_bytes(&v["input"]).unwrap();
        assert_eq!(bytes, v["expected_utf8"].as_str().unwrap().as_bytes());
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, v["expected_hex"].as_str().unwrap());
    }
}
#[test]
fn canonical_json_rejects_every_float_including_integral_floats() {
    for value in [
        json!(1.0),
        json!({"nested": [0.5]}),
        serde_json::from_str("1e3").unwrap(),
    ] {
        assert_eq!(
            canonical_bytes(&value).unwrap_err().code,
            ErrorCode::InvalidCanonicalValue
        );
    }
}
#[test]
fn count_measure_keeps_precision_and_coverage_separate_from_provenance() {
    let measure = CountMeasure {
        value: None,
        unit: "person".into(),
        precision: Precision {
            mode: PrecisionMode::Expected,
        },
        coverage: Coverage::Partial,
        basis: "fixture".into(),
        at_year: 1354,
    };
    let v = serde_json::to_value(&measure).unwrap();
    assert_eq!(v["value"], Value::Null);
    assert_eq!(v["precision"]["mode"], "expected");
    assert_eq!(v["coverage"], "partial");
    assert_eq!(serde_json::from_value::<CountMeasure>(v).unwrap(), measure);
}
#[test]
fn direct_event_lookup_matches_projection_without_provider() {
    let atlas = fixture_atlas();
    let contract = contract(ImpactMode::ExactCount { count: 3 });
    let catalogue = CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap();
    let resolution = resolve_impact(42, &contract, &catalogue).unwrap();
    // Deux calculs indépendants depuis la fixture : aucun résultat de requête
    // n'est utilisé comme ensemble de candidats pour le second parcours.
    let mut reverse_atlas = atlas.clone();
    reverse_atlas.settlements.reverse();
    let projected_resolution = resolve_impact(
        42,
        &contract,
        &CompleteCandidateCatalogue::from_atlas_sites(&reverse_atlas, "region").unwrap(),
    )
    .unwrap();
    let canon = BTreeMap::from([(
        contract.parent_event_id.clone(),
        event(
            &contract.parent_event_id,
            1350,
            EventPhase::ContextAndShocks,
            &[],
        ),
    )]);
    for target in &resolution.targets {
        let direct = &canon[&resolution.for_target(&target.target_id).unwrap().event_id];
        let projected = projected_resolution.project(&BTreeSet::from([target.target_id.clone()]));
        let projected = &canon[&projected[0].event_id];
        assert_eq!(direct.id, projected.id);
        assert_eq!(
            canonical_bytes(&serde_json::to_value(direct).unwrap()).unwrap(),
            canonical_bytes(&serde_json::to_value(projected).unwrap()).unwrap()
        );
    }
}
#[test]
fn impact_is_independent_of_target_enumeration_order() {
    let mut atlas = fixture_atlas();
    for mode in [
        ImpactMode::Probability {
            probability_bps: 2500,
        },
        ImpactMode::ExactCount { count: 3 },
    ] {
        let c = contract(mode);
        let before = resolve_impact(
            42,
            &c,
            &CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap(),
        )
        .unwrap();
        atlas.settlements.reverse();
        atlas.cells[0].settlement_summary.site_ids.reverse();
        let after = resolve_impact(
            42,
            &c,
            &CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap(),
        )
        .unwrap();
        assert_eq!(before, after);
    }
}
#[test]
fn exact_impact_uses_full_candidate_set() {
    let atlas = fixture_atlas();
    let c = contract(ImpactMode::ExactCount { count: 5 });
    let full = CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap();
    let r = resolve_impact(42, &c, &full).unwrap();
    let mut paged = vec![];
    for page in atlas.settlements.chunks(2).rev() {
        let ids = page.iter().map(|s| s.site_id.clone()).collect();
        paged.extend(r.project(&ids).into_iter().cloned());
    }
    paged.sort_by(|a, b| a.target_id.cmp(&b.target_id));
    assert_eq!(paged, r.targets);
    assert_eq!(r.targets.len(), 5);
    let partial = CompleteCandidateCatalogue::from_owner_catalogue(
        "region".into(),
        ImpactTarget::Sites,
        atlas.settlements[..2]
            .iter()
            .map(|s| ImpactCandidate {
                id: s.site_id.clone(),
                existence: PERIOD,
            })
            .collect(),
    )
    .unwrap();
    assert_eq!(
        resolve_impact(42, &c, &partial).unwrap_err().code,
        ErrorCode::ExactImpactInfeasible
    );
}
#[test]
fn exact_impact_attains_k_with_declared_rounding_or_fails() {
    let atlas = fixture_atlas();
    let full = CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap();
    for (mode, k) in [
        (ImpactMode::ExactCount { count: 0 }, 0),
        (ImpactMode::ExactCount { count: 12 }, 12),
        (
            ImpactMode::ExactFraction {
                numerator: 1,
                denominator: 5,
                rounding: RoundingRule::Floor,
            },
            2,
        ),
        (
            ImpactMode::ExactFraction {
                numerator: 1,
                denominator: 5,
                rounding: RoundingRule::Ceil,
            },
            3,
        ),
        (
            ImpactMode::FixedTarget {
                target_ref: "site/01".into(),
            },
            1,
        ),
    ] {
        assert_eq!(
            resolve_impact(42, &contract(mode), &full)
                .unwrap()
                .targets
                .len(),
            k
        );
    }
    assert_eq!(
        resolve_impact(42, &contract(ImpactMode::ExactCount { count: 13 }), &full)
            .unwrap_err()
            .code,
        ErrorCode::ExactImpactInfeasible
    );
}
#[test]
fn probability_impact_uses_independent_exposure_and_separate_intensity_keys() {
    let atlas = fixture_atlas();
    let c = contract(ImpactMode::Probability {
        probability_bps: 2500,
    });
    let full = CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap();
    let result = resolve_impact(42, &c, &full).unwrap();
    // On vérifie chaque Bernoulli ; aucune assertion de quota sur la somme.
    for s in atlas.settlements {
        let exposed =
            random_semantics_v2::uniform_below(42, &[&c.id, &s.site_id, "exposure"], 0, 10_000)
                .unwrap()
                < 2500;
        assert_eq!(result.for_target(&s.site_id).is_some(), exposed);
        if let Some(t) = result.for_target(&s.site_id) {
            assert_eq!(
                t.intensity_draw,
                random_semantics_v2::draw(42, &[&c.id, &s.site_id, "intensity"], 0, 0)
            );
        }
    }
}
#[test]
fn exact_demographic_impacts_and_oversized_catalogues_are_rejected() {
    for target in [ImpactTarget::People, ImpactTarget::Cohorts] {
        let full =
            CompleteCandidateCatalogue::from_owner_catalogue("region".into(), target, vec![])
                .unwrap();
        let mut c = contract(ImpactMode::ExactCount { count: 0 });
        c.target = target;
        assert_eq!(
            resolve_impact(42, &c, &full).unwrap_err().code,
            ErrorCode::UnsupportedCapability
        );
    }
    let full = CompleteCandidateCatalogue::from_owner_catalogue(
        "region".into(),
        ImpactTarget::Sites,
        (0..100_001)
            .map(|i| ImpactCandidate {
                id: i.to_string(),
                existence: PERIOD,
            })
            .collect(),
    )
    .unwrap();
    assert_eq!(
        resolve_impact(42, &contract(ImpactMode::ExactCount { count: 1 }), &full)
            .unwrap_err()
            .code,
        ErrorCode::BudgetExceeded
    );
}
#[test]
fn causal_cycle_is_rejected_with_canonical_dependency_cycle() {
    let events = vec![
        event("a", 1350, EventPhase::Deaths, &["b"]),
        event("b", 1350, EventPhase::Deaths, &["a"]),
    ];
    assert_eq!(
        causal_ranks(&events).unwrap_err().code,
        ErrorCode::CanonicalDependencyCycle
    );
    let events = vec![
        event("a", 1350, EventPhase::Deaths, &["b"]),
        event("b", 1351, EventPhase::Assignments, &["a"]),
    ];
    assert_eq!(
        causal_ranks(&events).unwrap_err().code,
        ErrorCode::CanonicalDependencyCycle
    );
}
#[test]
fn impact_contract_roundtrips_without_losing_mode_fields() {
    for mode in [
        ImpactMode::Probability {
            probability_bps: 2500,
        },
        ImpactMode::ExactCount { count: 3 },
        ImpactMode::ExactFraction {
            numerator: 1,
            denominator: 4,
            rounding: RoundingRule::Floor,
        },
        ImpactMode::FixedTarget {
            target_ref: "site/00".into(),
        },
    ] {
        let c = contract(mode);
        assert_eq!(
            serde_json::from_value::<ImpactContract>(serde_json::to_value(&c).unwrap()).unwrap(),
            c
        );
        let mut v = serde_json::to_value(&c).unwrap();
        v["unknown"] = json!(1);
        assert!(serde_json::from_value::<ImpactContract>(v).is_err());
    }
}
#[test]
fn same_year_cause_in_earlier_phase_is_accepted() {
    let events = vec![
        event("death", 1350, EventPhase::Deaths, &["shock"]),
        event("shock", 1350, EventPhase::ContextAndShocks, &[]),
    ];
    assert!(causal_ranks(&events).is_ok());
}
#[test]
fn intraphase_ranks_follow_dependencies_before_lexical_ids() {
    let mut events = vec![
        event("a", 1350, EventPhase::Deaths, &["z"]),
        event("z", 1350, EventPhase::Deaths, &[]),
        event("b", 1350, EventPhase::Deaths, &[]),
    ];
    let ranks = causal_ranks(&events).unwrap();
    assert!(ranks["z"] < ranks["a"]);
    events.reverse();
    assert_eq!(ranks, causal_ranks(&events).unwrap());
    events[0].causes.push("missing".into());
    assert!(causal_ranks(&events).is_err());
}
#[test]
fn future_causes_are_rejected() {
    for (year, phase) in [
        (1351, EventPhase::Bootstrap),
        (1350, EventPhase::Assignments),
    ] {
        assert_eq!(
            causal_ranks(&[
                event("a", 1350, EventPhase::Deaths, &["b"]),
                event("b", year, phase, &[])
            ])
            .unwrap_err()
            .code,
            ErrorCode::InvalidCausalOrder
        );
    }
}
fn relation(id: &str, a: &str, b: &str, predicate: Predicate) -> SocialRelation {
    SocialRelation {
        id: id.into(),
        subject: ObjectRef::Entity(a.into()),
        predicate,
        object: ObjectRef::Entity(b.into()),
        valid: PERIOD,
        role_id: None,
        weight_bps: None,
        time_cost_bps: None,
        cause_event_id: None,
        provenance: provenance(),
        rule_id: "test-1".into(),
    }
}
#[test]
fn strict_composition_is_acyclic_but_affiliations_may_overlap() {
    let mut relations = vec![
        relation("ab", "a", "b", Predicate::ComponentOf),
        relation("ba", "b", "a", Predicate::ComponentOf),
    ];
    assert_eq!(
        validate_composition(&relations).unwrap_err().code,
        ErrorCode::CompositionCycle
    );
    relations[1].predicate = Predicate::MemberOf;
    assert!(validate_composition(&relations).is_ok());
}
#[test]
fn half_open_life_and_coverage_exclude_death_and_empty_lives() {
    let mut p = PersonRecord {
        entity_id: "p".into(),
        provider_ref: "fixture".into(),
        birth_year: 1300,
        death_year: Some(1356),
        parent_refs: vec![],
        union_refs: vec![],
        residence_episodes: vec![],
        source: PersonSource::Author,
        ancestry_profile: "human".into(),
        social_attribute_episodes: vec![],
        coverage: PERIOD,
    };
    assert!(p.is_alive_and_covered(1355));
    assert!(!p.is_alive_and_covered(1356));
    p.death_year = Some(1300);
    assert!(!p.is_alive_and_covered(1300));
    let empty = TimeSpan {
        start_year: 1300,
        end_year: 1300,
    };
    assert!(empty.validate(true).is_ok());
    assert!(empty.validate(false).is_err());
    p.death_year = None;
    assert!(!p.is_alive_and_covered(1400));
}
#[test]
fn population_example_roundtrips_identically_in_canonical_json() {
    let p = example();
    assert_eq!(
        canonical_bytes(&example_value()).unwrap(),
        canonical_bytes(&serde_json::to_value(&p).unwrap()).unwrap()
    );
    validate_plan(&p, &references(&p)).unwrap();
}
#[test]
fn population_plan_rejects_unknown_fields_at_every_nesting_level() {
    for pointer in [
        "",
        "/provider",
        "/resolved_profiles/society",
        "/settlements/0",
        "/settlements/0/occupation_episodes/0",
        "/historical_program/0",
        "/limits",
    ] {
        let mut v = example_value();
        v.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(1));
        assert!(
            serde_json::from_value::<PopulationPlan>(v).is_err(),
            "{pointer}"
        );
    }
}
#[test]
fn population_plan_rejects_missing_required_nullable_and_null_optional_effects() {
    let mut v = example_value();
    v["boundary_ports"][0]
        .as_object_mut()
        .unwrap()
        .remove("bridge_plan_id");
    assert!(serde_json::from_value::<PopulationPlan>(v).is_err());
    let mut v = example_value();
    v["shocks"][0]["effects"]["additional_mortality_bps"] = Value::Null;
    assert!(serde_json::from_value::<PopulationPlan>(v).is_err());
}
fn reject_change(pointer: &str, value: Value) -> HistoryError {
    let base = example();
    let refs = references(&base);
    let mut v = example_value();
    *v.pointer_mut(pointer).unwrap() = value;
    let changed: PopulationPlan = serde_json::from_value(v).unwrap();
    let e = validate_plan(&changed, &refs).unwrap_err();
    assert_eq!(e.owner.as_deref(), Some(base.domain_id.as_str()));
    assert_eq!(e.plan_id.as_deref(), Some(base.plan_id.as_str()));
    e
}
#[test]
fn invalid_plan_references_are_rejected() {
    for pointer in [
        "/shocks/0/source_event_id",
        "/settlements/0/kinship_profile_id",
        "/settlements/0/history_culture_id",
        "/role_demands/0/office_id",
        "/mobility_edges/0/to_site_id",
        "/boundary_ports/0/contact_id",
        "/demography_society_id",
    ] {
        reject_change(pointer, json!("absent"));
    }
    reject_change("/settlements/1/provider_local_id", json!(0));
}
#[test]
fn invalid_plan_dates_are_rejected() {
    for (pointer, value) in [
        ("/timeline/end_year", 1300),
        ("/baseline_markers/0/year", 1400),
        ("/role_demands/0/valid/end_year", 1401),
        (
            "/settlements/0/occupation_episodes/0/valid/start_year",
            1301,
        ),
        ("/shocks/0/valid/end_year", 1354),
        ("/historical_program/0/year", 1300),
    ] {
        reject_change(pointer, json!(value));
    }
}
#[test]
fn invalid_plan_units_and_parameters_are_rejected() {
    for (pointer, value) in [
        ("/shocks/0/effects/additional_mortality_bps", 10001),
        ("/shocks/0/effects/fertility_multiplier_ppm", 10000001),
        ("/boundary_ports/0/rate_period_years", 2),
        ("/mobility_edges/0/travel_days", 0),
        (
            "/resolved_profiles/society/parameters/marriage_rate_bps",
            10001,
        ),
    ] {
        reject_change(pointer, json!(value));
    }
    let mut p = example();
    p.resolved_profiles
        .society
        .parameters
        .insert("unknown".into(), ParameterValue::Integer(1));
    assert!(validate_plan(&p, &references(&example())).is_err());
    reject_change("/seed", json!("42"));
    reject_change("/world_seed", json!("042"));
}
#[test]
fn invalid_plan_weights_are_rejected() {
    reject_change(
        "/settlements/0/ancestry_weights",
        json!({"human-example":0}),
    );
    reject_change(
        "/resolved_profiles/society/status_weights",
        json!({"absent":1}),
    );
    reject_change(
        "/settlements/0/occupation_episodes/0/activity_weights",
        json!({"agriculture":1000001}),
    );
    reject_change(
        "/resolved_profiles/social_classes/0/provider_rank",
        json!(1),
    );
}
#[test]
fn invalid_plan_budgets_are_rejected() {
    for (pointer, value) in [
        ("/limits/max_result_bytes", 536870913),
        ("/limits/max_candidate_slots", 250001),
        ("/limits/max_halo_contacts", 17),
        ("/limits/max_attempts", 3),
        ("/baseline_markers/0/population", 20001),
        ("/limits/core_target_share_bps", 1001),
    ] {
        reject_change(pointer, json!(value));
    }
    let p = example();
    let mut refs = references(&p);
    refs.site_population_expected_max
        .insert("site/demo/a".into(), 20_000);
    assert_eq!(
        validate_plan(&p, &refs).unwrap_err().code,
        ErrorCode::BudgetExceeded
    );
}
#[test]
fn unimplemented_plan_capabilities_are_typed_but_rejected() {
    for (pointer, value) in [
        ("/provider/mode", json!("procedural")),
        (
            "/provider/capabilities",
            json!(["historical_soft_v1", "conditional_people_v1"]),
        ),
        ("/settlements/0/foundation_year", json!(1320)),
        (
            "/settlements/0/occupation_episodes/0/state",
            json!("abandoned"),
        ),
        ("/historical_program/0/precision", json!("exact")),
        ("/role_demands/0/policy", json!("required")),
        ("/core_policy", json!("required_core")),
        (
            "/resolved_profiles/ancestries/0/reproduction_mode",
            json!("custom"),
        ),
    ] {
        assert_eq!(
            reject_change(pointer, value).code,
            ErrorCode::UnsupportedCapability
        );
    }
    let mut p = example();
    let refs = references(&p);
    p.boundary_ports[0].mode = PortMode::CommittedBridge;
    p.boundary_ports[0].bridge_plan_id = Some("bridge/1".into());
    assert_eq!(
        validate_plan(&p, &refs).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
    let mut v = example_value();
    v["historical_program"][0] = json!({"op_id":"op/force","source_event_id":"event/demo/plague","kind":"force_death","person_id":"p","year":1354});
    let p: PopulationPlan = serde_json::from_value(v).unwrap();
    assert_eq!(
        validate_plan(&p, &refs).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
    for op in [
        json!({"op_id":"op/protect","source_event_id":"event/demo/plague","kind":"protect_person","person_id":"p","valid":{"start_year":1354,"end_year":1357}}),
        json!({"op_id":"op/admit","source_event_id":"event/demo/displacement","kind":"admit_people","person_ids":["p"],"source_port_id":"port/demo/north","year":1365}),
    ] {
        let mut v = example_value();
        v["historical_program"][0] = op;
        let p: PopulationPlan = serde_json::from_value(v).unwrap();
        assert_eq!(
            validate_plan(&p, &refs).unwrap_err().code,
            ErrorCode::UnsupportedCapability
        );
    }
    let mut v = example_value();
    v["fixed_people"] = json!([{"person_id":"p","birth_year":1320,"death_year":null,"ancestry_profile_id":"human-example","sex":"F","parent_ids":[],"initial_site_id":"site/demo/a","protected_spans":[]}]);
    let p: PopulationPlan = serde_json::from_value(v).unwrap();
    assert_eq!(
        validate_plan(&p, &refs).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
    let mut v = example_value();
    v["exact_constraints"] = json!([{"constraint_id":"c","source_event_id":"event/demo/plague","kind":"population_at_date","year":1354,"site_ids":["site/demo/a"],"count":100,"phase":80,"effect_mode":"assert","count_basis":"state_at_date"}]);
    let p: PopulationPlan = serde_json::from_value(v).unwrap();
    assert_eq!(
        validate_plan(&p, &refs).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
}
#[test]
fn atlas_rejects_invalid_hierarchy_catalogues_routes_and_episodes() {
    let atlas = fixture_atlas();
    atlas.validate().unwrap();
    let mut a = atlas.clone();
    a.cells[0].parent_id = Some("region".into());
    assert!(a.validate().is_err());
    let mut a = atlas.clone();
    a.cells[0].settlement_summary.site_ids.pop();
    assert!(a.validate().is_err());
    let mut a = atlas.clone();
    a.settlements[0].occupation_episodes[0].valid.end_year = 1399;
    assert!(a.validate().is_err());
    let mut a = atlas.clone();
    a.mobility_edges.push(MobilityEdge {
        edge_id: "edge".into(),
        from_site_id: "site/00".into(),
        to_site_id: "missing".into(),
        valid: PERIOD,
        travel_days: 1,
        bidirectional: true,
        mode: MobilityMode::Land,
    });
    assert!(a.validate().is_err());
    let mut a = atlas;
    a.settlements[0].coordinate_frame = "other".into();
    assert!(a.validate().is_err());
}
