//! Régressions S0 : seed 42, chronos-social-2, schéma de plan 1.
use chronos_core::v2::{
    atlas::*, canonical::canonical_bytes, ids::*, model::*, population::*, region::*, validate::*,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const WORLD: &str = "canon-s0-seed42";
const PERIOD: TimeSpan = TimeSpan {
    start_year: 1300,
    end_year: 1400,
};
/// Seuils du §27.2, recopiés ici : `engine_defaults.alignment` relève de S1.
const THRESHOLDS: ConductThresholds = ConductThresholds {
    lawless_morality_max_bps: -6000,
    lawless_lawfulness_max_bps: -3000,
    crooked_morality_max_bps: -2500,
    ordinary_morality_below_bps: 2500,
    upright_morality_below_bps: 7000,
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
        end_year: None,
        phase,
        causes: causes.iter().map(|s| (*s).into()).collect(),
        participants: vec![],
        provenance: provenance(),
    }
}
fn site_ids(range: std::ops::Range<usize>) -> Vec<String> {
    range.map(|i| format!("site/{i:02}")).collect()
}
fn cell(id: &str, parent: Option<&str>, neighbors: &[&str], sites: Vec<String>) -> AtlasCell {
    AtlasCell {
        cell_id: id.into(),
        parent_id: parent.map(String::from),
        neighbors: neighbors.iter().map(|s| (*s).into()).collect(),
        area_m2: 1000,
        bounds: MetricBounds {
            min_x_mm: 0,
            min_y_mm: 0,
            max_x_mm: 12,
            max_y_mm: 10,
        },
        coordinate_frame: "local-mm".into(),
        settlement_summary: SettlementSummary {
            population_expected_max: 200 * sites.len() as u64,
            site_ids: sites,
        },
    }
}
/// Une région et deux cellules filles, six sites dans chacune.
fn fixture_atlas() -> HistoryAtlas {
    let settlements: Vec<_> = (0..12)
        .map(|i| SettlementAnchor {
            site_id: format!("site/{i:02}"),
            cell_id: if i < 6 {
                "region/north"
            } else {
                "region/south"
            }
            .into(),
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
        cells: vec![
            cell("region", None, &[], site_ids(0..12)),
            cell(
                "region/north",
                Some("region"),
                &["region/south"],
                site_ids(0..6),
            ),
            cell(
                "region/south",
                Some("region"),
                &["region/north"],
                site_ids(6..12),
            ),
        ],
        region_ids: vec!["region".into()],
        settlements,
        mobility_edges: vec![],
        external_contacts: vec![],
    }
}
fn event_address(cell: &str, kind: &str, block: i32, ordinal: u32) -> SemanticAddress {
    SemanticAddress::event(&Owner::cell(WORLD, cell), kind, block, ordinal).unwrap()
}
fn contract_on(
    id: &str,
    owner: &str,
    parent: &SemanticAddress,
    mode: ImpactMode,
) -> ImpactContract {
    ImpactContract {
        id: id.into(),
        parent_event_id: parent.encode(),
        owner_scope: owner.into(),
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
fn contract(mode: ImpactMode) -> ImpactContract {
    let parent = event_address("region", "war", 1350, 0);
    contract_on("impact/test/0", "region", &parent, mode)
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
fn variant(family: &str, society: &str) -> CultureVariant {
    CultureVariant {
        history_family_id: family.into(),
        demography_society_id: society.into(),
    }
}
fn site_end(port: &BoundaryPort) -> ContactSiteEnd {
    ContactSiteEnd {
        site_id: port.entry_site_id.clone(),
        external_context_id: port.external_context_id.clone(),
        access_edge_id: None,
        external_profile_id: port.external_profile_id.clone(),
        population_expected: port.population_expected,
    }
}
fn references(p: &PopulationPlan) -> PlanReferences {
    PlanReferences {
        canon_id: p.canon_id.clone(),
        timeline: p.timeline,
        world_seed: p.world_seed.parse().unwrap(),
        catalogue_fingerprints: p.catalogue_fingerprints.clone(),
        event_years: BTreeMap::from([
            ("event/demo/plague".into(), 1354),
            ("event/demo/displacement".into(), 1365),
        ]),
        office_ids: BTreeSet::from(["office/demo/mayor-a".into()]),
        culture_variants: BTreeMap::from([
            (
                "european-organic".into(),
                variant("agrarian_estates", &p.demography_society_id),
            ),
            (
                "bastide".into(),
                variant("agrarian_estates", &p.demography_society_id),
            ),
            ("other-culture".into(), variant("other", "society/other")),
        ]),
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
                        ends: [
                            ContactEnd::Site(site_end(port)),
                            ContactEnd::External {
                                context_id: port.external_context_id.clone(),
                            },
                        ],
                        valid: port.valid,
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
fn domain_key() -> DomainKey {
    DomainKey::new(WORLD, "region", "society", 0).unwrap()
}

#[test]
fn address_roundtrip_preserves_every_field_and_type() {
    for (kind, fields) in [
        (
            AddressKind::Entity,
            vec!["w", "cell", "scope", "office", "-10", "mayor"],
        ),
        (
            AddressKind::Person,
            vec!["w", "region", "society", "0", "algorithm-1", "17"],
        ),
        (
            AddressKind::Tenure,
            vec!["w", "cell", "scope", "office", "1350", "mayor", "0"],
        ),
        (
            AddressKind::Event,
            vec![
                "w", "domain", "region", "society", "0", "plague", "1350", "2",
            ],
        ),
        (
            AddressKind::Group,
            vec!["w", "cell", "scope", "group_slot", "farm", "1350", "2"],
        ),
        (
            AddressKind::Household,
            vec![
                "w",
                "region",
                "society",
                "0",
                "household",
                "singleton/17",
                "1354",
            ],
        ),
        (
            AddressKind::Transfer,
            vec!["w", "cell", "scope", "transfer", "a", "b", "1354", "2"],
        ),
        (AddressKind::Plan, vec!["w", "region", "society", "0"]),
        (AddressKind::Domain, vec!["w", "region", "society", "0"]),
    ] {
        let address =
            SemanticAddress::from_fields(kind, fields.into_iter().map(String::from).collect())
                .unwrap();
        assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
        assert!(!address.encode().contains('='));
    }
    let domain = domain_key();
    let a = SemanticAddress::person(&domain, "algo", "1").unwrap();
    let b = SemanticAddress::person(&domain, "algo", "2").unwrap();
    let owner = Owner::Domain(domain);
    let relation = SemanticAddress::relation(&owner, Predicate::ParentOf, &a, &b, -5, 0).unwrap();
    assert_eq!(
        SemanticAddress::decode(&relation.encode()).unwrap(),
        relation
    );
    assert!(!relation.fields().contains(&a.encode()));
    assert_ne!(a.encode(), b.encode());
}
#[test]
fn person_tenure_and_plan_have_one_identity_per_object() {
    let domain = domain_key();
    let person = SemanticAddress::person(&domain, "algo", "17").unwrap();
    // Toute construction d'une même personne donne le même ID, et l'ancienne
    // forme à portée jointe n'est plus une adresse.
    assert_eq!(
        person,
        SemanticAddress::from_fields(AddressKind::Person, person.fields().to_vec()).unwrap()
    );
    let joined = vec![
        WORLD.into(),
        "region/society/0".into(),
        "algo".into(),
        "17".into(),
    ];
    assert!(SemanticAddress::from_fields(AddressKind::Person, joined).is_err());
    let plan = vec![WORLD.to_string(), "region/society/0".to_string()];
    assert!(SemanticAddress::from_fields(AddressKind::Plan, plan).is_err());
    let tenure = vec![WORLD.to_string(), "office".to_string(), "0".to_string()];
    assert!(SemanticAddress::from_fields(AddressKind::Tenure, tenure).is_err());
}
#[test]
fn address_longer_than_4096_characters_is_rejected() {
    assert!(SemanticAddress::person(&domain_key(), "a", &"x".repeat(4096)).is_err());
    assert!(SemanticAddress::decode(&format!("person.{}", "A".repeat(4096))).is_err());
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
fn canonical_json_refuses_every_shared_rejection_vector() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../docs/design/chronos-social-v2/vectors/canonical-json.v1.json"
    ))
    .unwrap();
    let rejections = vectors["rejections"].as_array().unwrap();
    assert!(!rejections.is_empty());
    for r in rejections {
        let text = r["input_text"].as_str().unwrap();
        // Refus au décodage du texte ou à la canonisation, jamais d'octets.
        if let Ok(value) = serde_json::from_str::<Value>(text) {
            assert!(canonical_bytes(&value).is_err(), "{}", r["name"]);
        }
    }
    for bound in ["minimum", "maximum"] {
        let text = vectors["integer_bounds"][bound].as_str().unwrap();
        let value: Value = serde_json::from_str(text).unwrap();
        assert_eq!(canonical_bytes(&value).unwrap(), text.as_bytes());
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
    assert_eq!(
        serde_json::from_value::<CountMeasure>(v.clone()).unwrap(),
        measure
    );
    // Valeur inconnue : `null` explicite, jamais un champ absent (§4.3).
    let mut missing = v;
    missing.as_object_mut().unwrap().remove("value");
    assert!(serde_json::from_value::<CountMeasure>(missing).is_err());
}
/// Preuve de sortie S0 (§20.2), seed 42, sans fournisseur : accès direct par
/// cible et projection paginée d'une résolution indépendante donnent les mêmes
/// faits, sur un événement-cadre, deux batailles filles et une épidémie.
#[test]
fn direct_event_lookup_matches_projection_without_provider() {
    let atlas = fixture_atlas();
    let war = event_address("region", "war", 1350, 0);
    let battle_north = event_address("region/north", "battle", 1350, 0);
    let battle_south = event_address("region/south", "battle", 1350, 1);
    let plague = event_address("region", "plague", 1350, 1);
    let mut frame = event(&war.encode(), 1352, EventPhase::ContextAndShocks, &[]);
    frame.end_year = Some(1360);
    let canon = vec![
        frame,
        event(
            &battle_north.encode(),
            1354,
            EventPhase::ContextAndShocks,
            &[&war.encode()],
        ),
        event(
            &battle_south.encode(),
            1355,
            EventPhase::ContextAndShocks,
            &[&war.encode()],
        ),
        event(&plague.encode(), 1354, EventPhase::ContextAndShocks, &[]),
    ];
    causal_ranks(&canon).unwrap();
    let by_id: BTreeMap<_, _> = canon.iter().map(|e| (e.id.as_str(), e)).collect();
    let probability = |probability_bps| ImpactMode::Probability { probability_bps };
    let contracts = [
        contract_on(
            "impact/war",
            "region",
            &war,
            ImpactMode::ExactCount { count: 5 },
        ),
        contract_on(
            "impact/north",
            "region/north",
            &battle_north,
            probability(5000),
        ),
        contract_on(
            "impact/south",
            "region/south",
            &battle_south,
            ImpactMode::ExactCount { count: 3 },
        ),
        contract_on("impact/plague", "region", &plague, probability(4000)),
    ];
    let mut reverse_atlas = atlas.clone();
    reverse_atlas.settlements.reverse();
    let mut direct = BTreeMap::new();
    let mut projected = BTreeMap::new();
    for c in &contracts {
        let owner = c.owner_scope.as_str();
        let ids: Vec<String> = atlas
            .settlements
            .iter()
            .filter(|s| atlas.cell_is_within(&s.cell_id, owner))
            .map(|s| s.site_id.clone())
            .collect();
        // Accès direct : une question par site, exposé ou non.
        let catalogue = CompleteCandidateCatalogue::from_atlas_sites(&atlas, owner).unwrap();
        let resolution = resolve_impact(42, c, &catalogue).unwrap();
        for id in &ids {
            if let Some(hit) = resolution.for_target(id) {
                assert_eq!(&hit.target_id, id);
                let key = (hit.event_id.clone(), id.clone());
                direct.insert(key, (hit.effect_id.clone(), hit.intensity_draw));
            }
        }
        assert!(
            !resolution.targets.is_empty() && resolution.targets.len() < ids.len(),
            "{}",
            c.id
        );
        // Projection : autre résolution, autre ordre d'atlas, pages inversées.
        let catalogue =
            CompleteCandidateCatalogue::from_atlas_sites(&reverse_atlas, owner).unwrap();
        let resolution = resolve_impact(42, c, &catalogue).unwrap();
        for page in ids.chunks(2).rev() {
            let page: BTreeSet<String> = page.iter().cloned().collect();
            for hit in resolution.project(&page) {
                assert!(page.contains(&hit.target_id), "{}", hit.target_id);
                let key = (hit.event_id.clone(), hit.target_id.clone());
                let value = (hit.effect_id.clone(), hit.intensity_draw);
                assert!(projected.insert(key, value).is_none());
            }
        }
    }
    assert_eq!(direct, projected);
    // Un effet par cible et par événement, fils de son événement.
    let effects: BTreeSet<_> = direct.values().map(|(id, _)| id).collect();
    assert_eq!(effects.len(), direct.len());
    let draws: BTreeSet<_> = direct.values().map(|(_, draw)| draw).collect();
    assert_eq!(draws.len(), direct.len());
    for ((event_id, _), (effect_id, _)) in &direct {
        let parent = SemanticAddress::decode(event_id).unwrap();
        let effect = SemanticAddress::decode(effect_id).unwrap();
        assert_eq!(effect.kind(), AddressKind::Effect);
        assert_eq!(&effect.fields()[..parent.fields().len()], parent.fields());
        assert!(by_id.contains_key(event_id.as_str()));
    }
    // Parent et enfant restent deux faits : un même site touché par les deux
    // porte deux effets et deux intensités.
    let on_war: BTreeSet<_> = direct
        .keys()
        .filter(|(e, _)| e == &war.encode())
        .map(|(_, site)| site)
        .collect();
    let children = [battle_north.encode(), battle_south.encode()];
    let shared = direct
        .iter()
        .find(|((e, site), _)| children.contains(e) && on_war.contains(site))
        .expect("site exposé au parent et à un enfant");
    assert_ne!(&direct[&(war.encode(), shared.0 .1.clone())], shared.1);
    for child in [&battle_north, &battle_south] {
        assert_eq!(by_id[child.encode().as_str()].causes, [war.encode()]);
    }
    assert_eq!(by_id[war.encode().as_str()].end_year, Some(1360));
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
fn sites_as_candidates(atlas: &HistoryAtlas) -> Vec<ImpactCandidate> {
    atlas
        .settlements
        .iter()
        .map(|s| ImpactCandidate {
            id: s.site_id.clone(),
            existence: PERIOD,
        })
        .collect()
}
#[test]
fn exact_impact_uses_full_candidate_set() {
    let atlas = fixture_atlas();
    let c = contract(ImpactMode::ExactCount { count: 5 });
    let full = CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap();
    let r = resolve_impact(42, &c, &full).unwrap();
    // Classement refait ici sur les douze sites : priorité, puis ID.
    let mut ranked: Vec<_> = atlas
        .settlements
        .iter()
        .map(|s| {
            let parts = [c.id.as_str(), s.site_id.as_str(), "priority"];
            (
                random_semantics_v2::draw(42, &parts, 0, 0),
                s.site_id.clone(),
            )
        })
        .collect();
    ranked.sort();
    let mut expected: Vec<_> = ranked[..5].iter().map(|(_, id)| id.clone()).collect();
    expected.sort();
    let selected: Vec<_> = r.targets.iter().map(|t| t.target_id.clone()).collect();
    assert_eq!(selected, expected);
    // Toutes les paginations recomposent exactement le même résultat.
    let ids: Vec<_> = atlas
        .settlements
        .iter()
        .map(|s| s.site_id.clone())
        .collect();
    for size in [1, 2, 5, 12] {
        let mut paged = vec![];
        for page in ids.chunks(size).rev() {
            let page: BTreeSet<String> = page.iter().cloned().collect();
            let hits = r.project(&page);
            assert!(hits.iter().all(|t| page.contains(&t.target_id)));
            paged.extend(hits.into_iter().cloned());
        }
        paged.sort_by(|a, b| a.target_id.cmp(&b.target_id));
        assert_eq!(paged, r.targets, "pages de {size}");
    }
    // Une page ne devient jamais un catalogue : la preuve du propriétaire la
    // refuse, même si K y tiendrait.
    let proof = CatalogueProof::for_atlas_sites(&atlas, "region").unwrap();
    let all = sites_as_candidates(&atlas);
    assert_eq!(
        CompleteCandidateCatalogue::from_owner_catalogue(&proof, all[..6].to_vec())
            .unwrap_err()
            .code,
        ErrorCode::IncompleteCatalogue
    );
}
#[test]
fn owner_catalogue_requires_a_proof_bound_to_its_owner() {
    let atlas = fixture_atlas();
    let proof = CatalogueProof::for_atlas_sites(&atlas, "region").unwrap();
    let all = sites_as_candidates(&atlas);
    let c = contract(ImpactMode::ExactCount { count: 5 });
    let accepted = CompleteCandidateCatalogue::from_owner_catalogue(&proof, all.clone()).unwrap();
    assert_eq!(
        accepted,
        CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap()
    );
    assert!(resolve_impact(42, &c, &accepted).is_ok());
    let mut substituted = all.clone();
    substituted[0].id = "site/autre".into();
    let mut duplicated = all.clone();
    duplicated[1] = duplicated[0].clone();
    let mut shifted = all.clone();
    shifted[0].existence.start_year += 1;
    let mut longer = all.clone();
    longer.push(ImpactCandidate {
        id: "site/en-trop".into(),
        existence: PERIOD,
    });
    for list in [all[..11].to_vec(), substituted, duplicated, shifted, longer] {
        let e = CompleteCandidateCatalogue::from_owner_catalogue(&proof, list).unwrap_err();
        assert_eq!(e.code, ErrorCode::IncompleteCatalogue);
        assert_eq!(e.owner.as_deref(), Some("region"));
    }
    // La preuve d'un propriétaire ne couvre ni un autre propriétaire ni un
    // autre type de cible, même avec la bonne liste.
    let mut other_owner = proof.clone();
    other_owner.owner_scope = "region/north".into();
    let mut other_target = proof.clone();
    other_target.target = ImpactTarget::Assets;
    for forged in [other_owner, other_target] {
        assert_eq!(
            CompleteCandidateCatalogue::from_owner_catalogue(&forged, all.clone())
                .unwrap_err()
                .code,
            ErrorCode::IncompleteCatalogue
        );
    }
    // Le catalogue d'une cellule fille ne sert pas un contrat de la région.
    let north = CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region/north").unwrap();
    assert_eq!(
        resolve_impact(42, &c, &north).unwrap_err().code,
        ErrorCode::InvalidImpactContract
    );
}
#[test]
fn impact_draws_are_pinned_by_literal_values() {
    // Oracle : port Python indépendant de keys.rs. Changer `exposure`,
    // `priority`, `intensity` ou l'ordre des parties fait échouer ce test.
    let atlas = fixture_atlas();
    let full = CompleteCandidateCatalogue::from_atlas_sites(&atlas, "region").unwrap();
    let selected = |mode| {
        resolve_impact(42, &contract(mode), &full)
            .unwrap()
            .targets
            .into_iter()
            .map(|t| t.target_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        selected(ImpactMode::ExactCount { count: 5 }),
        ["site/00", "site/01", "site/05", "site/09", "site/10"]
    );
    assert_eq!(
        selected(ImpactMode::Probability {
            probability_bps: 2500
        }),
        ["site/00", "site/06", "site/10"]
    );
    assert_eq!(
        selected(ImpactMode::Probability {
            probability_bps: 5000
        }),
        ["site/00", "site/02", "site/05", "site/06", "site/10"]
    );
    let r = resolve_impact(42, &contract(ImpactMode::ExactCount { count: 5 }), &full).unwrap();
    assert_eq!(
        r.for_target("site/00").unwrap().intensity_draw,
        11969561304100620598
    );
    assert_eq!(
        r.for_target("site/01").unwrap().intensity_draw,
        15264254884598781494
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
fn frame_event_end_is_explicit_and_after_its_start() {
    let mut frame = event("frame", 1350, EventPhase::ContextAndShocks, &[]);
    frame.end_year = Some(1360);
    assert!(causal_ranks(std::slice::from_ref(&frame)).is_ok());
    let v = serde_json::to_value(&frame).unwrap();
    assert_eq!(
        serde_json::from_value::<HistoryEvent>(v.clone()).unwrap(),
        frame
    );
    let mut missing = v;
    missing.as_object_mut().unwrap().remove("end_year");
    assert!(serde_json::from_value::<HistoryEvent>(missing).is_err());
    for end in [1350, 1349, 100_001] {
        frame.end_year = Some(end);
        let e = causal_ranks(std::slice::from_ref(&frame)).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidCausalOrder);
        assert_eq!(e.at_year, Some(1350));
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
fn duplicate_relation_id_is_not_reported_as_a_cycle() {
    let r = relation("ab", "a", "b", Predicate::MemberOf);
    assert_eq!(
        validate_composition(&[r.clone(), r]).unwrap_err().code,
        ErrorCode::InvalidCanonicalValue
    );
}
fn person() -> PersonRecord {
    PersonRecord {
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
        observed_span: PERIOD,
        alignment_episodes: vec![],
    }
}
#[test]
fn half_open_life_and_observation_exclude_death_and_empty_lives() {
    let mut p = person();
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
fn person_record_keeps_unknown_death_and_observation_distinct_from_coverage() {
    let mut p = person();
    p.death_year = None;
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["death_year"], Value::Null);
    assert!(v.get("coverage").is_none());
    assert_eq!(v["observed_span"]["end_year"], 1400);
    let mut missing = v;
    missing.as_object_mut().unwrap().remove("death_year");
    assert!(serde_json::from_value::<PersonRecord>(missing).is_err());
}
#[test]
fn parent_refs_are_table_rows_or_typed_bounded_external_refs() {
    let mut p = person();
    p.parent_refs = vec![
        ParentRef::Slot {
            person_id: "row/1".into(),
        },
        ParentRef::External {
            origin: ExternalParentOrigin::ExternalArrival,
            ref_id: "arrival/1".into(),
        },
    ];
    p.validate().unwrap();
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["parent_refs"][1]["kind"], "external");
    assert_eq!(v["parent_refs"][1]["origin"], "external_arrival");
    assert_eq!(serde_json::from_value::<PersonRecord>(v).unwrap(), p);
    let mut unbounded = p.clone();
    unbounded.parent_refs[1] = ParentRef::External {
        origin: ExternalParentOrigin::CoverageLimit,
        ref_id: "x".repeat(4097),
    };
    let mut three = p.clone();
    three.parent_refs.push(ParentRef::Slot {
        person_id: "row/3".into(),
    });
    let mut twice = p.clone();
    twice.parent_refs[1] = twice.parent_refs[0].clone();
    let mut too_old = p;
    too_old.birth_year = -100_001;
    for invalid in [unbounded, three, twice, too_old] {
        let e = invalid.validate().unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidCanonicalValue);
        assert_eq!(e.owner.as_deref(), Some("p"));
    }
    assert!(serde_json::from_value::<ParentRef>(json!({"kind":"entity","id":"p"})).is_err());
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
fn population_example_identity_is_derived_from_its_domain() {
    let p = example();
    let domain = DomainKey::from_id(&p.domain_id).unwrap();
    assert_eq!(domain.world(), p.canon_id);
    assert_eq!(domain.region(), "region/demo");
    assert_eq!(domain.society(), p.demography_society_id);
    assert_eq!(domain.ordinal(), 0);
    assert_eq!(SemanticAddress::plan(&domain).unwrap().encode(), p.plan_id);
    // Oracle : port Python indépendant de keys.rs.
    assert_eq!(p.seed, "5854947233154073793");
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
fn with_plan_identity(p: &mut PopulationPlan, domain: &DomainKey) {
    p.domain_id = SemanticAddress::domain(domain).unwrap().encode();
    p.plan_id = SemanticAddress::plan(domain).unwrap().encode();
    let world_seed = p.world_seed.parse().unwrap();
    let seed = random_semantics_v2::domain_seed(world_seed, &p.domain_id, &p.provider.algorithm_id);
    p.seed = seed.to_string();
}
#[test]
fn plan_is_bound_to_the_canon_and_the_world_horizon() {
    let p = example();
    let good = references(&p);
    let invalid_plan = |plan: &PopulationPlan, refs: &PlanReferences| {
        assert_eq!(
            validate_plan(plan, refs).unwrap_err().code,
            ErrorCode::InvalidPopulationPlan
        );
    };
    let mut refs = good.clone();
    refs.canon_id = "canon-autre".into();
    invalid_plan(&p, &refs);
    let mut refs = good.clone();
    refs.world_seed += 1;
    invalid_plan(&p, &refs);
    // Horizon plus long ou plus court que le plan : jamais un plan partiel.
    for end_year in [1399, 1401] {
        let mut refs = good.clone();
        refs.timeline.end_year = end_year;
        invalid_plan(&p, &refs);
    }
    // plan_id étranger : autre ordinal, forme lisible, type d'adresse erroné.
    let domain = DomainKey::from_id(&p.domain_id).unwrap();
    let other = DomainKey::new(domain.world(), domain.region(), domain.society(), 1).unwrap();
    for plan_id in [
        SemanticAddress::plan(&other).unwrap().encode(),
        "pop/demo/domain-0/v1".to_string(),
        p.domain_id.clone(),
    ] {
        let mut q = p.clone();
        q.plan_id = plan_id;
        invalid_plan(&q, &good);
    }
    // Domaine cohérent avec le plan_id mais d'un autre monde ou d'un autre
    // profil, graine recalculée : la cohérence interne ne suffit pas.
    for foreign in [
        DomainKey::new("canon-autre", domain.region(), domain.society(), 0).unwrap(),
        DomainKey::new(domain.world(), domain.region(), "society/autre", 0).unwrap(),
    ] {
        let mut q = p.clone();
        with_plan_identity(&mut q, &foreign);
        invalid_plan(&q, &good);
    }
    let mut q = p.clone();
    q.domain_id = "domain/demo/0".into();
    invalid_plan(&q, &good);
    // Un autre domaine du même monde reste un plan valide s'il est complet.
    let mut q = p.clone();
    with_plan_identity(&mut q, &other);
    validate_plan(&q, &good).unwrap();
}
fn capability_error(p: &PopulationPlan, refs: &PlanReferences) {
    assert_eq!(
        validate_plan(p, refs).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
}
#[test]
fn mixed_society_profiles_are_refused_as_a_capability() {
    let p = example();
    let refs = references(&p);
    // Variante principale d'un autre profil, y compris sans poids d'épisode.
    let mut mixed = refs.clone();
    mixed.culture_variants.insert(
        "bastide".into(),
        variant("agrarian_estates", "society/other"),
    );
    capability_error(&p, &mixed);
    // Variante secondaire de poids positif d'un épisode.
    let mut q = p.clone();
    q.settlements[0].occupation_episodes[0]
        .culture_weights
        .insert("other-culture".into(), 1);
    capability_error(&q, &refs);
    // Changement culturel porté par un épisode ultérieur.
    let mut q = p.clone();
    let site = &mut q.settlements[1];
    let mut later = site.occupation_episodes[0].clone();
    site.occupation_episodes[0].valid.end_year = 1350;
    later.valid.start_year = 1350;
    later.culture_weights = BTreeMap::from([("other-culture".into(), 10_000)]);
    site.occupation_episodes.push(later);
    capability_error(&q, &refs);
}
#[test]
fn zero_weight_variant_does_not_need_the_domain_profile() {
    let p = example();
    let refs = references(&p);
    let mut q = p.clone();
    q.settlements[0].occupation_episodes[0]
        .culture_weights
        .insert("other-culture".into(), 0);
    validate_plan(&q, &refs).unwrap();
    // Toutes les variantes de poids positif partagent le profil : accepté.
    let mut q = p.clone();
    q.settlements[0].occupation_episodes[0]
        .culture_weights
        .insert("bastide".into(), 500);
    validate_plan(&q, &refs).unwrap();
    // Même de poids nul, une variante absente du raccord reste une référence
    // cassée, pas une question de capacité.
    let mut q = p;
    q.settlements[0].occupation_episodes[0]
        .culture_weights
        .insert("absente".into(), 0);
    assert_eq!(
        validate_plan(&q, &refs).unwrap_err().code,
        ErrorCode::InvalidPopulationPlan
    );
}
#[test]
fn principal_variant_must_belong_to_its_declared_family() {
    for culture in [
        "autre_famille/bastide",
        "agrarian_estates/absente",
        "agrarian_estates/bastide/extra",
        "/bastide",
        "bastide",
    ] {
        assert_eq!(
            reject_change("/settlements/0/history_culture_id", json!(culture)).code,
            ErrorCode::InvalidPopulationPlan,
            "{culture}"
        );
    }
}
#[test]
fn plan_canonical_bytes_ignore_identity_fields_and_list_order() {
    let p = example();
    let reference = plan_canonical_bytes(&p).unwrap();
    let value: Value = serde_json::from_slice(&reference).unwrap();
    assert!(value.get("plan_id").is_none());
    assert!(value.get("plan_fingerprint").is_none());
    assert_eq!(value["canon_id"], p.canon_id);
    let mut q = p.clone();
    q.plan_id = "plan.autre".into();
    q.plan_fingerprint = "autre".into();
    assert_eq!(plan_canonical_bytes(&q).unwrap(), reference);
    let mut q = p.clone();
    q.provider.capabilities.reverse();
    q.resolved_profiles.social_classes.reverse();
    q.settlements.reverse();
    q.baseline_markers.reverse();
    q.role_demands[0].status_ids.reverse();
    assert_ne!(q, p);
    assert_eq!(plan_canonical_bytes(&q).unwrap(), reference);
    // Listes triées par octets UTF-8 dans le résultat.
    let ids: Vec<_> = value["resolved_profiles"]["social_classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["commoner", "elite", "notable", "skilled"]);
    assert_eq!(value["provider"]["capabilities"][0], "candidate_slots_v1");
    // Un vrai changement de fait change les octets.
    let mut q = p.clone();
    q.settlements[0].initial_population += 1;
    assert_ne!(plan_canonical_bytes(&q).unwrap(), reference);
    // Une liste ordonnée n'est pas retriée : l'ordre des épisodes reste un fait.
    let mut q = p;
    let site = &mut q.settlements[0];
    let mut later = site.occupation_episodes[0].clone();
    site.occupation_episodes[0].valid.end_year = 1350;
    later.valid.start_year = 1350;
    site.occupation_episodes.push(later);
    let ordered = plan_canonical_bytes(&q).unwrap();
    q.settlements[0].occupation_episodes.reverse();
    assert_ne!(plan_canonical_bytes(&q).unwrap(), ordered);
}
#[test]
fn kinship_profile_must_match_the_society_profile() {
    for (pointer, value) in [
        ("/resolved_profiles/kinship/0/dependent_age", 16),
        ("/resolved_profiles/society/parameters/dependent_age", 21),
    ] {
        assert_eq!(
            reject_change(pointer, json!(value)).code,
            ErrorCode::InvalidPopulationPlan,
            "{pointer}"
        );
    }
    // Majorité commune mais parent encore dépendant.
    let mut p = example();
    let refs = references(&p);
    p.resolved_profiles.kinship[0].dependent_age = 21;
    p.resolved_profiles
        .society
        .parameters
        .insert("dependent_age".into(), ParameterValue::Integer(21));
    assert_eq!(
        validate_plan(&p, &refs).unwrap_err().code,
        ErrorCode::InvalidPopulationPlan
    );
    p.resolved_profiles.kinship[0].min_parent_age = 21;
    validate_plan(&p, &refs).unwrap();
    // Un second profil incompatible n'est pas masqué par le premier.
    let mut other = p.resolved_profiles.kinship[0].clone();
    other.id = "second-profile".into();
    other.dependent_age = 18;
    other.min_parent_age = 18;
    p.resolved_profiles.kinship.push(other);
    assert!(validate_plan(&p, &refs).is_err());
    // Modèle de ménage inconnu : refusé dès la lecture.
    let mut v = example_value();
    v["resolved_profiles"]["kinship"][0]["household_model"] = json!("clan_compound_v1");
    assert!(serde_json::from_value::<PopulationPlan>(v).is_err());
}
#[test]
fn residence_accepts_the_three_lineage_lab_modes_only() {
    let p = example();
    let refs = references(&p);
    for mode in ["patrilocal", "matrilocal", "either"] {
        let mut q = p.clone();
        q.resolved_profiles
            .society
            .parameters
            .insert("residence".into(), ParameterValue::Text(mode.into()));
        validate_plan(&q, &refs).unwrap();
    }
    assert_eq!(
        reject_change(
            "/resolved_profiles/society/parameters/residence",
            json!("neolocal")
        )
        .code,
        ErrorCode::InvalidPopulationPlan
    );
}
#[test]
fn second_side_of_a_contact_opens_a_port_under_the_same_contact_id() {
    let p = example();
    let mut refs = references(&p);
    let first = p.boundary_ports[0].clone();
    let mut second = first.clone();
    second.port_id = "port/demo/north-b".into();
    second.entry_site_id = "site/demo/b".into();
    second.external_context_id = "outside/demo/north-b".into();
    second.population_expected = 2500;
    let contact = refs.contacts.get_mut(&first.contact_id).unwrap();
    contact.ends = [
        ContactEnd::Site(site_end(&first)),
        ContactEnd::Site(site_end(&second)),
    ];
    // Le premier côté garde son port, le second est accepté sous le même ID.
    validate_plan(&p, &refs).unwrap();
    let mut q = p.clone();
    q.boundary_ports[0] = second.clone();
    validate_plan(&q, &refs).unwrap();
    let mut q = p.clone();
    q.boundary_ports.push(second.clone());
    validate_plan(&q, &refs).unwrap();
    // Un port doit reprendre ce que déclare son extrémité.
    let mut q = p.clone();
    let mut wrong = second.clone();
    wrong.population_expected = 1;
    q.boundary_ports[0] = wrong;
    assert!(validate_plan(&q, &refs).is_err());
    // Un site qui n'est pas une extrémité n'ouvre aucun port.
    let contact = refs.contacts.get_mut(&first.contact_id).unwrap();
    contact.ends[1] = ContactEnd::External {
        context_id: first.external_context_id.clone(),
    };
    let mut q = p;
    q.boundary_ports[0] = second;
    assert!(validate_plan(&q, &refs).is_err());
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
#[test]
fn atlas_errors_carry_their_owner_and_date() {
    let mut a = fixture_atlas();
    a.region_ids.clear();
    let e = a.validate().unwrap_err();
    assert_eq!(e.code, ErrorCode::InvalidAtlas);
    assert_eq!(e.owner.as_deref(), Some("atlas-s0"));
    assert_eq!(e.at_year, Some(1300));
    let mut a = fixture_atlas();
    a.timeline.end_year = 1300;
    let e = a.validate().unwrap_err();
    assert_eq!(e.code, ErrorCode::InvalidInterval);
    assert_eq!(e.owner.as_deref(), Some("atlas-s0"));
    assert_eq!(e.at_year, Some(1300));
}
#[test]
fn atlas_ids_and_coordinates_are_bounded() {
    let limit = 9_000_000_000_000_000;
    let mut a = fixture_atlas();
    for c in &mut a.cells {
        c.bounds.min_x_mm = -limit;
        c.bounds.max_x_mm = limit;
    }
    a.validate().unwrap();
    for (min, max) in [(-limit - 1, limit), (-limit, limit + 1)] {
        let mut b = a.clone();
        for c in &mut b.cells {
            c.bounds.min_x_mm = min;
            c.bounds.max_x_mm = max;
        }
        assert!(b.validate().is_err());
    }
    let mut a = fixture_atlas();
    a.atlas_id = "atlas-\u{e9}t\u{e9}".into();
    assert!(a.validate().is_err());
    let mut a = fixture_atlas();
    a.settlements[0].site_id = "site 00".into();
    a.cells[0].settlement_summary.site_ids[0] = "site 00".into();
    a.cells[1].settlement_summary.site_ids[0] = "site 00".into();
    assert!(a.validate().is_err());
    let mut a = fixture_atlas();
    a.settlements[0].foundation_year = -100_001;
    assert!(a.validate().is_err());
}
fn atlas_with_contact(ends: [ContactEnd; 2]) -> HistoryAtlas {
    let mut a = fixture_atlas();
    a.external_contacts.push(ExternalContact {
        contact_id: "contact/0".into(),
        owner_scope: "region".into(),
        ends,
        valid: PERIOD,
    });
    a
}
fn contact_site(site: &str, context: &str) -> ContactEnd {
    ContactEnd::Site(ContactSiteEnd {
        site_id: site.into(),
        external_context_id: context.into(),
        access_edge_id: None,
        external_profile_id: "outside-profile".into(),
        population_expected: 100,
    })
}
#[test]
fn external_contact_carries_two_valid_ends() {
    let outside = ContactEnd::External {
        context_id: "outside/0".into(),
    };
    let site_a = contact_site("site/00", "outside/0");
    let site_b = contact_site("site/07", "outside/1");
    atlas_with_contact([site_a.clone(), outside.clone()])
        .validate()
        .unwrap();
    let a = atlas_with_contact([site_a.clone(), site_b.clone()]);
    a.validate().unwrap();
    let contact = &a.external_contacts[0];
    assert!(contact.site_end("site/00").is_some());
    assert!(contact.site_end("site/07").is_some());
    assert!(contact.site_end("site/01").is_none());
    for ends in [
        // Ordre non canonique, extrémité répétée, aucun site, contexte divergent,
        // site inconnu.
        [site_b.clone(), site_a.clone()],
        [site_a.clone(), site_a.clone()],
        [
            ContactEnd::External {
                context_id: "outside/0".into(),
            },
            ContactEnd::External {
                context_id: "outside/1".into(),
            },
        ],
        [contact_site("site/00", "outside/2"), outside.clone()],
        [contact_site("site/99", "outside/0"), outside.clone()],
    ] {
        assert!(atlas_with_contact(ends).validate().is_err());
    }
    let v = serde_json::to_value(contact).unwrap();
    assert_eq!(v["ends"][0]["kind"], "site");
    assert_eq!(
        serde_json::from_value::<ExternalContact>(v).unwrap(),
        *contact
    );
}
fn axes(morality: i32, lawfulness: i32) -> AlignmentAxes {
    AlignmentAxes::new(morality, lawfulness).unwrap()
}
#[test]
fn conduct_class_is_derived_from_axes() {
    use ConductClass::*;
    for (morality, lawfulness, class) in [
        // Chaque frontière du §27.2, de part et d'autre.
        (-10_000, -10_000, Lawless),
        (-6000, -3000, Lawless),
        (-5999, -3000, Crooked),
        (-6000, -2999, Crooked),
        (-10_000, 10_000, Crooked),
        (-2500, 0, Crooked),
        (-2499, 0, Ordinary),
        (0, -10_000, Ordinary),
        (2499, 0, Ordinary),
        (2500, 0, Upright),
        (6999, 0, Upright),
        (7000, 0, Saintly),
        (10_000, -10_000, Saintly),
    ] {
        assert_eq!(
            conduct_class(axes(morality, lawfulness), &THRESHOLDS),
            class,
            "{morality}/{lawfulness}"
        );
    }
    // Les seuils sont un paramètre : la fonction n'en fige aucun.
    let strict = ConductThresholds {
        upright_morality_below_bps: 9000,
        ..THRESHOLDS
    };
    assert_eq!(conduct_class(axes(7000, 0), &strict), Upright);
}
#[test]
fn conduct_class_applies_to_people_and_groups_but_not_to_categories() {
    let a = axes(-7000, -5000);
    for nature in [
        EntityNature::Person,
        EntityNature::OrganizedGroup,
        EntityNature::Community,
    ] {
        assert_eq!(
            entity_conduct_class(nature, a, &THRESHOLDS),
            Some(ConductClass::Lawless)
        );
    }
    for nature in [EntityNature::Category, EntityNature::Population] {
        assert_eq!(entity_conduct_class(nature, a, &THRESHOLDS), None);
    }
}
#[test]
fn alignment_axes_out_of_bounds_are_refused() {
    assert!(AlignmentAxes::new(10_000, -10_000).is_ok());
    for (m, l) in [(10_001, 0), (-10_001, 0), (0, 10_001), (0, -10_001)] {
        assert_eq!(
            AlignmentAxes::new(m, l).unwrap_err().code,
            ErrorCode::InvalidAlignment
        );
        let v = json!({"morality_bps": m, "lawfulness_bps": l});
        assert!(serde_json::from_value::<AlignmentAxes>(v).is_err());
    }
    let v = json!({"morality_bps": 1, "lawfulness_bps": 2});
    assert_eq!(
        serde_json::from_value::<AlignmentAxes>(v).unwrap(),
        axes(1, 2)
    );
    assert!(serde_json::from_value::<AlignmentAxes>(json!({"morality_bps": 1})).is_err());
    let v = json!({"morality_bps": 1, "lawfulness_bps": 2, "class": "saintly"});
    assert!(serde_json::from_value::<AlignmentAxes>(v).is_err());
}
#[test]
fn entity_state_stores_axes_and_tags_but_never_the_conduct_class() {
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
    let state = EntityState {
        entity_id: "e".into(),
        at_year: 1354,
        lifecycle: Lifecycle::Active,
        member_count: measure,
        cohesion_bps: 6000,
        legitimacy_bps: 5000,
        wealth_index: 0,
        capabilities: vec![],
        alignment: axes(-7000, -5000),
        conduct_tags: BTreeSet::from([ConductTag::Violence, ConductTag::Extortion]),
        visibility: Visibility::Covert,
        goals: vec![],
        assets: vec![],
        footprint_refs: vec![],
        institution_version: "1".into(),
    };
    let v = serde_json::to_value(&state).unwrap();
    assert!(v.get("conduct_class").is_none());
    assert_eq!(v["visibility"], "covert");
    // Tags triés par ID, comme toute liste canonique.
    assert_eq!(v["conduct_tags"], json!(["extortion", "violence"]));
    assert_eq!(serde_json::from_value::<EntityState>(v).unwrap(), state);
    let all = BTreeSet::from([
        ConductTag::Violence,
        ConductTag::Usury,
        ConductTag::Sorcery,
        ConductTag::Extortion,
        ConductTag::Charity,
        ConductTag::Blasphemy,
    ]);
    let names = serde_json::to_value(&all).unwrap();
    let names: Vec<_> = names.as_array().unwrap().iter().collect();
    assert!(names.windows(2).all(|w| w[0].as_str() < w[1].as_str()));
    assert!(serde_json::from_value::<ConductTag>(json!("inconnu")).is_err());
}
fn alignment_episode(start: i32, end: i32, cause: Option<&str>) -> AlignmentEpisode {
    AlignmentEpisode {
        valid: TimeSpan {
            start_year: start,
            end_year: end,
        },
        axes: axes(0, 0),
        conduct_tags: BTreeSet::new(),
        cause_event_id: cause.map(String::from),
        provenance: provenance(),
    }
}
#[test]
fn alignment_change_cites_an_event_and_episodes_are_ordered() {
    let mut p = person();
    p.alignment_episodes = vec![
        alignment_episode(1300, 1340, None),
        alignment_episode(1340, 1356, Some("event/conversion")),
    ];
    p.validate().unwrap();
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(serde_json::from_value::<PersonRecord>(v).unwrap(), p);
    let mut uncaused = p.clone();
    uncaused.alignment_episodes[1].cause_event_id = None;
    let mut overlapping = p;
    overlapping.alignment_episodes[1].valid.start_year = 1339;
    for invalid in [uncaused, overlapping] {
        assert_eq!(
            invalid.validate().unwrap_err().code,
            ErrorCode::InvalidAlignment
        );
    }
}
fn standing(
    id: &str,
    subject: &str,
    jurisdiction: &str,
    status: LegalStatus,
    (start_year, end_year): (i32, i32),
    cause: Option<&str>,
) -> LegalStanding {
    LegalStanding {
        id: id.into(),
        subject: ObjectRef::Entity(subject.into()),
        jurisdiction: ObjectRef::Entity(jurisdiction.into()),
        status,
        valid: TimeSpan {
            start_year,
            end_year,
        },
        cause_event_id: cause.map(String::from),
        provenance: provenance(),
        rule_id: "s0-test-1".into(),
    }
}
#[test]
fn legal_standing_is_relative_to_jurisdiction() {
    use LegalStatus::*;
    let edict = {
        let mut e = event("edict/1", 1350, EventPhase::InstitutionsAndVacancies, &[]);
        e.kind = "edict".into();
        e
    };
    let standings = [
        standing("a", "cult", "city/a", Licit, (1300, 1350), None),
        standing(
            "b",
            "cult",
            "city/a",
            Proscribed,
            (1350, 1400),
            Some("edict/1"),
        ),
        standing("c", "cult", "city/b", Tolerated, (1300, 1400), None),
    ];
    validate_legal_standings(&standings, std::slice::from_ref(&edict)).unwrap();
    let at = |jurisdiction: &str, year| {
        legal_status_at(
            &standings,
            &ObjectRef::Entity("cult".into()),
            &ObjectRef::Entity(jurisdiction.into()),
            year,
        )
    };
    // Même sujet, même date, deux statuts ; intervalle semi-ouvert.
    assert_eq!(at("city/a", 1349), Licit);
    assert_eq!(at("city/a", 1350), Proscribed);
    assert_eq!(at("city/b", 1350), Tolerated);
    // Aucune fiche dans une juridiction, ou hors de la période : licite.
    assert_eq!(at("city/c", 1350), Licit);
    assert_eq!(at("city/a", 1400), Licit);
    // Sujets légaux admis, juridiction qui gouverne seulement.
    let mut activity = standings[2].clone();
    activity.subject = ObjectRef::Activity("usury".into());
    let mut person_slot = standings[2].clone();
    person_slot.subject = ObjectRef::PersonSlot("slot/1".into());
    validate_legal_standings(&[activity.clone()], &[]).unwrap();
    validate_legal_standings(&[person_slot], &[]).unwrap();
    let mut site_subject = activity.clone();
    site_subject.subject = ObjectRef::Site("site/00".into());
    let mut activity_jurisdiction = activity;
    activity_jurisdiction.jurisdiction = ObjectRef::Activity("usury".into());
    for invalid in [site_subject, activity_jurisdiction] {
        assert_eq!(
            validate_legal_standings(&[invalid], &[]).unwrap_err().code,
            ErrorCode::InvalidLegalStanding
        );
    }
    // Conduite et légalité restent indépendantes : une résistance proscrite
    // peut être vertueuse.
    assert_eq!(
        conduct_class(axes(8000, -8000), &THRESHOLDS),
        ConductClass::Saintly
    );
    let v = serde_json::to_value(&standings[0]).unwrap();
    assert_eq!(v["status"], "licit");
    assert_eq!(
        serde_json::from_value::<LegalStanding>(v.clone()).unwrap(),
        standings[0]
    );
    let mut missing = v;
    missing.as_object_mut().unwrap().remove("cause_event_id");
    assert!(serde_json::from_value::<LegalStanding>(missing).is_err());
}
#[test]
fn standing_change_cites_an_event() {
    use LegalStatus::*;
    let of_kind = |id: &str, kind: &str, year| {
        let mut e = event(id, year, EventPhase::InstitutionsAndVacancies, &[]);
        e.kind = kind.into();
        e
    };
    for kind in LEGAL_CHANGE_EVENT_KINDS {
        let events = [of_kind("cause", kind, 1350)];
        let chain = [
            standing("a", "g", "j", Licit, (1300, 1350), None),
            standing("b", "g", "j", Proscribed, (1350, 1400), Some("cause")),
        ];
        validate_legal_standings(&chain, &events).unwrap();
    }
    let events = [
        of_kind("edict", "edict", 1350),
        of_kind("late", "charter", 1351),
        of_kind("war", "war", 1350),
    ];
    let initial = standing("a", "g", "j", Licit, (1300, 1350), None);
    for (cause, message) in [
        (None, "sans événement"),
        (Some("absent"), "événement absent"),
        (Some("late"), "événement postérieur"),
        (Some("war"), "autre type d'événement"),
    ] {
        let change = standing("b", "g", "j", Proscribed, (1350, 1400), cause);
        assert_eq!(
            validate_legal_standings(&[change, initial.clone()], &events)
                .unwrap_err()
                .code,
            ErrorCode::InvalidLegalStanding,
            "{message}"
        );
    }
    // Deux statuts simultanés dans la même juridiction.
    let overlap = standing("b", "g", "j", Proscribed, (1349, 1400), Some("edict"));
    assert!(validate_legal_standings(&[initial.clone(), overlap], &events).is_err());
    // Une autre juridiction ouvre sa propre chaîne, avec son état initial.
    let elsewhere = standing("b", "g", "k", Proscribed, (1350, 1400), None);
    validate_legal_standings(&[initial, elsewhere], &events).unwrap();
}
fn entity_value() -> Value {
    let address = event_address("region", "war", 1350, 0);
    json!({
        "id": "entity/0",
        "semantic_address": address,
        "kind": "guild",
        "archetype_id": "base_workshop",
        "nature": "organized_group",
        "agency": "governed",
        "owner_scope": "region",
        "existence": {"start_year": 1300, "end_year": 1400},
        "culture_refs": [],
        "institution_ref": null,
        "provenance": provenance(),
    })
}
#[test]
fn name_tree_is_absent_by_default_and_kept_as_opaque_canonical_json() {
    let entity: SocialEntity = serde_json::from_value(entity_value()).unwrap();
    assert_eq!(entity.name_tree, None);
    let v = serde_json::to_value(&entity).unwrap();
    assert!(v.get("name_tree").is_none());
    let tree = json!({"rule": "guild", "parts": [{"text": "Marchands", "rank": 2}, null, true]});
    let mut v = entity_value();
    v["name_tree"] = tree.clone();
    let entity: SocialEntity = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(entity.name_tree.as_ref().unwrap().value(), &tree);
    let written = serde_json::to_value(&entity).unwrap();
    assert_eq!(
        canonical_bytes(&written).unwrap(),
        canonical_bytes(&v).unwrap()
    );
    // L'absence s'écrit en omettant le champ : `null` aurait une seconde forme.
    let mut v = entity_value();
    v["name_tree"] = Value::Null;
    assert!(serde_json::from_value::<SocialEntity>(v).is_err());
}
#[test]
fn entity_with_a_float_in_its_name_tree_is_refused() {
    for tree in [
        json!(1.5),
        json!({"weight": 1.0}),
        json!({"parts": [{"text": "x", "weight": 0.5}]}),
    ] {
        let mut v = entity_value();
        v["name_tree"] = tree.clone();
        assert!(serde_json::from_value::<SocialEntity>(v).is_err(), "{tree}");
        // Même refus en construction directe : aucun chemin ne contourne le
        // contrôle canonique.
        assert_eq!(
            NameTree::new(tree).unwrap_err().code,
            ErrorCode::InvalidCanonicalValue
        );
    }
}
