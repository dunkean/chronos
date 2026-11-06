//! Régressions S0 : seed 42, chronos-social-2, schéma de plan 1.
use chronos_core::v2::{
    atlas::*,
    canonical::{canonical_bytes, canonical_list_bytes, CanonicalRecord},
    ids::*,
    model::*,
    population::*,
    region::*,
    validate::*,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Nom d'une liste et permutation qui la retourne.
type Permutation<T> = (&'static str, fn(&mut T));
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
fn fact(
    address: &SemanticAddress,
    kind: &str,
    year: i32,
    phase: EventPhase,
    causes: Vec<String>,
) -> HistoryEvent {
    HistoryEvent {
        id: address.encode(),
        owner_scope: address.owner().unwrap().scope().clone(),
        kind: kind.into(),
        year,
        end_year: None,
        phase,
        causes,
        participants: vec![],
        provenance: provenance(),
    }
}
fn event_id(name: &str) -> String {
    event_address("region", name, 0, 0).encode()
}
/// Événement de fixture : son nom est son type, ses causes sont d'autres noms.
fn event(name: &str, year: i32, phase: EventPhase, causes: &[&str]) -> HistoryEvent {
    let address = event_address("region", name, 0, 0);
    let causes = causes.iter().map(|c| event_id(c)).collect();
    fact(&address, name, year, phase, causes)
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
            site_tags: BTreeSet::new(),
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
        provenance: provenance(),
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
fn region_owner() -> Owner {
    Owner::cell(WORLD, "region")
}
fn event_address(cell: &str, kind: &str, block: i32, ordinal: u32) -> SemanticAddress {
    SemanticAddress::event(&Owner::cell(WORLD, cell), kind, block, ordinal).unwrap()
}
fn contract_on(slot: &str, parent: &SemanticAddress, mode: ImpactMode) -> ImpactContract {
    ImpactContract {
        id: SemanticAddress::impact(parent, slot).unwrap().encode(),
        parent_event_id: parent.encode(),
        owner_scope: parent.owner().unwrap().scope().clone(),
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
    contract_on("impact-0", &event_address("region", "war", 1350, 0), mode)
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
        domain_sites: BTreeMap::from([(
            p.domain_id.clone(),
            p.settlements.iter().map(|s| s.site_id.clone()).collect(),
        )]),
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
                        owner_scope: OwnerScope::cell("region/demo"),
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
            AddressKind::Event,
            vec!["w", "world", "plague", "1350", "2"],
        ),
        (
            AddressKind::Event,
            vec!["w", "site", "site/a", "fire", "1350", "2"],
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
        (AddressKind::Site, vec!["w", "site/a"]),
        (AddressKind::Activity, vec!["w", "agriculture"]),
        (
            AddressKind::Asset,
            vec!["w", "cell", "scope", "asset", "mill", "1350", "x"],
        ),
    ] {
        let address =
            SemanticAddress::from_fields(kind, fields.into_iter().map(String::from).collect())
                .unwrap();
        assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
        assert!(!address.encode().contains('='));
    }
    let domain = domain_key();
    let a = ObjectRef::PersonSlot(person_id("1"));
    let b = ObjectRef::PersonSlot(person_id("2"));
    let owner = Owner::domain(&domain);
    let relation = SemanticAddress::relation(&owner, Predicate::ParentOf, &a, &b, -5, 0).unwrap();
    assert_eq!(
        SemanticAddress::decode(&relation.encode()).unwrap(),
        relation
    );
    assert!(!relation.fields().contains(&person_id("1")));
    assert_ne!(a, b);
}
#[test]
fn owner_scope_has_four_typed_json_forms() {
    for (scope, value) in [
        (OwnerScope::World, json!({"kind": "world"})),
        (
            OwnerScope::cell("region"),
            json!({"kind": "cell", "cell_id": "region"}),
        ),
        (
            OwnerScope::site("site/00"),
            json!({"kind": "site", "site_id": "site/00"}),
        ),
        (
            Owner::domain(&domain_key()).scope().clone(),
            json!({"kind": "domain", "region": "region", "society": "society", "ordinal": 0}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&scope).unwrap(), value);
        assert_eq!(serde_json::from_value::<OwnerScope>(value).unwrap(), scope);
    }
    for invalid in [
        json!("region"),
        json!({"kind": "planet"}),
        json!({"kind": "cell", "cell_id": "region", "extra": 1}),
        json!({"kind": "domain", "region": "r", "society": "s"}),
    ] {
        assert!(serde_json::from_value::<OwnerScope>(invalid).is_err());
    }
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
    let phase = EventPhase::ContextAndShocks;
    let mut frame = fact(&war, "war", 1352, phase, vec![]);
    frame.end_year = Some(1360);
    let canon = vec![
        frame,
        fact(&battle_north, "battle", 1354, phase, vec![war.encode()]),
        fact(&battle_south, "battle", 1355, phase, vec![war.encode()]),
        fact(&plague, "plague", 1354, phase, vec![]),
    ];
    causal_ranks(&canon).unwrap();
    let by_id: BTreeMap<_, _> = canon.iter().map(|e| (e.id.as_str(), e)).collect();
    let probability = |probability_bps| ImpactMode::Probability { probability_bps };
    let contracts = [
        (
            "region",
            contract_on("war", &war, ImpactMode::ExactCount { count: 5 }),
        ),
        (
            "region/north",
            contract_on("north", &battle_north, probability(5000)),
        ),
        (
            "region/south",
            contract_on("south", &battle_south, ImpactMode::ExactCount { count: 3 }),
        ),
        ("region", contract_on("plague", &plague, probability(4000))),
    ];
    let mut reverse_atlas = atlas.clone();
    reverse_atlas.settlements.reverse();
    let mut direct = BTreeMap::new();
    let mut projected = BTreeMap::new();
    for (cell, c) in &contracts {
        let owner = Owner::cell(WORLD, cell);
        let ids: Vec<String> = atlas
            .settlements
            .iter()
            .filter(|s| atlas.cell_is_within(&s.cell_id, cell))
            .map(|s| s.site_id.clone())
            .collect();
        // Accès direct : une question par site, exposé ou non.
        let catalogue = CompleteCandidateCatalogue::from_atlas(&atlas, &owner).unwrap();
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
        let catalogue = CompleteCandidateCatalogue::from_atlas(&reverse_atlas, &owner).unwrap();
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
        assert_eq!(effect.owner(), parent.owner());
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
            &CompleteCandidateCatalogue::from_atlas(&atlas, &region_owner()).unwrap(),
        )
        .unwrap();
        atlas.settlements.reverse();
        atlas.cells[0].settlement_summary.site_ids.reverse();
        let after = resolve_impact(
            42,
            &c,
            &CompleteCandidateCatalogue::from_atlas(&atlas, &region_owner()).unwrap(),
        )
        .unwrap();
        assert_eq!(before, after);
    }
}
#[test]
fn exact_impact_uses_full_candidate_set() {
    let atlas = fixture_atlas();
    let c = contract(ImpactMode::ExactCount { count: 5 });
    let full = CompleteCandidateCatalogue::from_atlas(&atlas, &region_owner()).unwrap();
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
    // Un atlas tronqué ne produit aucun catalogue : la liste vient de l'autorité,
    // et un résumé de cellule qui ne colle plus la fait refuser.
    let mut truncated = atlas.clone();
    truncated.settlements.truncate(6);
    assert_eq!(
        CompleteCandidateCatalogue::from_atlas(&truncated, &region_owner())
            .unwrap_err()
            .code,
        ErrorCode::InvalidAtlas
    );
}
fn catalogue_size(atlas: &HistoryAtlas, owner: &Owner) -> usize {
    let parent = SemanticAddress::event(owner, "survey", 1350, 0).unwrap();
    let c = contract_on(
        "survey",
        &parent,
        ImpactMode::Probability {
            probability_bps: 10_000,
        },
    );
    let catalogue = CompleteCandidateCatalogue::from_atlas(atlas, owner).unwrap();
    resolve_impact(42, &c, &catalogue).unwrap().targets.len()
}
#[test]
fn catalogue_enumerates_candidates_from_the_atlas_for_each_owner() {
    let atlas = fixture_atlas();
    assert_eq!(catalogue_size(&atlas, &Owner::whole_world(WORLD)), 12);
    assert_eq!(catalogue_size(&atlas, &region_owner()), 12);
    assert_eq!(
        catalogue_size(&atlas, &Owner::cell(WORLD, "region/north")),
        6
    );
    assert_eq!(catalogue_size(&atlas, &Owner::site(WORLD, "site/03")), 1);
    let refused = |owner: &Owner| {
        CompleteCandidateCatalogue::from_atlas(&atlas, owner)
            .unwrap_err()
            .code
    };
    // La partition des domaines relève de S2 : l'atlas n'en sait rien.
    assert_eq!(
        refused(&Owner::domain(&domain_key())),
        ErrorCode::UnsupportedCapability
    );
    assert_eq!(
        refused(&Owner::cell(WORLD, "absente")),
        ErrorCode::InvalidAtlas
    );
    assert_eq!(
        refused(&Owner::site(WORLD, "absent")),
        ErrorCode::InvalidAtlas
    );
}
#[test]
fn impact_contract_identity_is_bound_to_parent_owner_and_catalogue() {
    let atlas = fixture_atlas();
    let full = CompleteCandidateCatalogue::from_atlas(&atlas, &region_owner()).unwrap();
    let exact = ImpactMode::ExactCount { count: 2 };
    let good = contract(exact.clone());
    good.validate().unwrap();
    let refused = |c: &ImpactContract, catalogue: &CompleteCandidateCatalogue| {
        assert_eq!(
            resolve_impact(42, c, catalogue).unwrap_err().code,
            ErrorCode::InvalidImpactContract
        );
    };
    // `id` n'est pas une adresse d'impact, ou pas celle de ce parent.
    let mut readable = good.clone();
    readable.id = "impact/test/0".into();
    let mut event_as_id = good.clone();
    event_as_id.id = good.parent_event_id.clone();
    let mut other_parent = good.clone();
    other_parent.parent_event_id = event_address("region", "plague", 1350, 1).encode();
    let mut other_owner = good.clone();
    other_owner.owner_scope = OwnerScope::cell("region/north");
    for c in [&readable, &event_as_id, &other_parent, &other_owner] {
        assert!(c.validate().is_err());
        refused(c, &full);
    }
    // Contrat cohérent en lui-même mais d'un autre canon que le catalogue.
    let foreign = SemanticAddress::event(&Owner::cell("canon-autre", "region"), "war", 1350, 0);
    let foreign = contract_on("impact-0", &foreign.unwrap(), exact.clone());
    foreign.validate().unwrap();
    refused(&foreign, &full);
    // Contrat cohérent d'un autre propriétaire que le catalogue.
    let north = event_address("region/north", "battle", 1350, 0);
    let north = contract_on("impact-0", &north, exact);
    north.validate().unwrap();
    refused(&north, &full);
    let cell = Owner::cell(WORLD, "region/north");
    let north_catalogue = CompleteCandidateCatalogue::from_atlas(&atlas, &cell).unwrap();
    resolve_impact(42, &north, &north_catalogue).unwrap();
    refused(&good, &north_catalogue);
    // Les effets partagent canon et propriétaire avec leur parent.
    for target in resolve_impact(42, &good, &full).unwrap().targets {
        let effect = SemanticAddress::decode(&target.effect_id).unwrap();
        assert_eq!(effect.owner(), Some(region_owner()));
        assert_eq!(effect.fields()[0], WORLD);
    }
}
#[test]
fn impact_draws_are_pinned_by_literal_values() {
    // Oracle : port Python indépendant de keys.rs. Changer `exposure`,
    // `priority`, `intensity` ou l'ordre des parties fait échouer ce test.
    let atlas = fixture_atlas();
    let full = CompleteCandidateCatalogue::from_atlas(&atlas, &region_owner()).unwrap();
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
        ["site/05", "site/07", "site/08", "site/09", "site/11"]
    );
    assert_eq!(
        selected(ImpactMode::Probability {
            probability_bps: 2500
        }),
        ["site/00", "site/01", "site/04"]
    );
    assert_eq!(
        selected(ImpactMode::Probability {
            probability_bps: 5000
        }),
        ["site/00", "site/01", "site/02", "site/04", "site/05", "site/06", "site/11"]
    );
    let r = resolve_impact(42, &contract(ImpactMode::ExactCount { count: 5 }), &full).unwrap();
    assert_eq!(
        r.for_target("site/05").unwrap().intensity_draw,
        7138363109131796565
    );
    assert_eq!(
        r.for_target("site/07").unwrap().intensity_draw,
        8833795781382205987
    );
}
#[test]
fn exact_impact_attains_k_with_declared_rounding_or_fails() {
    let atlas = fixture_atlas();
    let full = CompleteCandidateCatalogue::from_atlas(&atlas, &region_owner()).unwrap();
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
    let full = CompleteCandidateCatalogue::from_atlas(&atlas, &region_owner()).unwrap();
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
    assert!(ranks[&event_id("z")] < ranks[&event_id("a")]);
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
fn guild(slot: &str) -> SemanticAddress {
    SemanticAddress::entity(&region_owner(), "guild", 1300, slot).unwrap()
}
fn guild_ref(slot: &str) -> ObjectRef {
    ObjectRef::Entity(guild(slot).encode())
}
fn link(subject: ObjectRef, predicate: Predicate, object: ObjectRef) -> SocialRelation {
    let id = SemanticAddress::relation(&region_owner(), predicate, &subject, &object, 1300, 0);
    SocialRelation {
        id: id.unwrap().encode(),
        subject,
        predicate,
        object,
        valid: PERIOD,
        role_id: None,
        weight_bps: None,
        time_cost_bps: None,
        cause_event_id: None,
        provenance: provenance(),
        rule_id: "test-1".into(),
    }
}
fn relation(a: &str, b: &str, predicate: Predicate) -> SocialRelation {
    link(guild_ref(a), predicate, guild_ref(b))
}
#[test]
fn strict_composition_is_acyclic_but_affiliations_may_overlap() {
    let cycle = [
        relation("a", "b", Predicate::ComponentOf),
        relation("b", "a", Predicate::ComponentOf),
    ];
    assert_eq!(
        validate_composition(&cycle).unwrap_err().code,
        ErrorCode::CompositionCycle
    );
    let overlap = [
        relation("a", "b", Predicate::ComponentOf),
        relation("b", "a", Predicate::MemberOf),
    ];
    assert!(validate_composition(&overlap).is_ok());
}
#[test]
fn duplicate_relation_id_is_not_reported_as_a_cycle() {
    let r = relation("a", "b", Predicate::MemberOf);
    assert_eq!(
        validate_composition(&[r.clone(), r]).unwrap_err().code,
        ErrorCode::InvalidCanonicalValue
    );
}
#[test]
fn relation_identity_is_proved_against_its_address() {
    let good = relation("a", "b", Predicate::Trusts);
    good.validate().unwrap();
    let mut predicate = good.clone();
    predicate.predicate = Predicate::Helps;
    let mut subject = good.clone();
    subject.subject = guild_ref("c");
    let mut object = good.clone();
    object.object = guild_ref("c");
    let mut start = good.clone();
    start.valid.start_year = 1301;
    let mut readable = good.clone();
    readable.id = "ab".into();
    let mut other_kind = good.clone();
    other_kind.id = guild("a").encode();
    for invalid in [predicate, subject, object, start, readable, other_kind] {
        assert_eq!(
            invalid.validate().unwrap_err().code,
            ErrorCode::InvalidRecordIdentity
        );
        assert_eq!(
            validate_composition(&[invalid]).unwrap_err().code,
            ErrorCode::InvalidRecordIdentity
        );
    }
}
#[test]
fn relation_constructor_accepts_every_addressable_object_ref() {
    let owner = region_owner();
    let site = SemanticAddress::site(WORLD, "site/00").unwrap();
    let activity = SemanticAddress::activity(WORLD, "agriculture").unwrap();
    let asset = SemanticAddress::asset(&owner, "mill", 1300, "north").unwrap();
    let office = SemanticAddress::entity(&owner, "office", 1300, "mayor").unwrap();
    let group = SemanticAddress::group(&owner, "farm", 1300, 0).unwrap();
    let household = SemanticAddress::household(&domain_key(), "singleton/1", 1300).unwrap();
    let refs = [
        guild_ref("a"),
        ObjectRef::Entity(group.encode()),
        ObjectRef::Entity(household.encode()),
        ObjectRef::Office(office.encode()),
        ObjectRef::Site(site.encode()),
        ObjectRef::Asset(asset.encode()),
        ObjectRef::PersonSlot(person_id("1")),
        ObjectRef::Activity(activity.encode()),
    ];
    for object in &refs {
        let r = link(guild_ref("owner"), Predicate::Owns, object.clone());
        r.validate().unwrap();
        let decoded = SemanticAddress::decode(&r.id).unwrap();
        let parts = decoded.link().unwrap();
        assert_eq!(parts.owner, owner);
        assert_eq!(parts.predicate, Some(Predicate::Owns));
        assert_eq!(&ObjectRef::from_address(&parts.object).unwrap(), object);
        assert_eq!(&object.address().unwrap(), &parts.object);
        assert_eq!(parts.start, 1300);
    }
    // Une forme canonique par objet : un office, une personne ou un site
    // déguisés sous une autre variante sont refusés.
    for invalid in [
        ObjectRef::Entity(office.encode()),
        ObjectRef::Entity(person_id("1")),
        ObjectRef::Office(guild("a").encode()),
        ObjectRef::Site(activity.encode()),
        ObjectRef::Asset(site.encode()),
        ObjectRef::PersonSlot(guild("a").encode()),
        ObjectRef::Activity("agriculture".into()),
        ObjectRef::Entity(event_id("war")),
    ] {
        assert!(invalid.address().is_err(), "{invalid:?}");
        assert!(SemanticAddress::relation(
            &owner,
            Predicate::Owns,
            &guild_ref("owner"),
            &invalid,
            1300,
            0
        )
        .is_err());
    }
}
fn person_id(local: &str) -> String {
    SemanticAddress::person(&domain_key(), "algo", local)
        .unwrap()
        .encode()
}
fn tags() -> BTreeSet<String> {
    ["charity", "extortion", "violence"]
        .into_iter()
        .map(String::from)
        .collect()
}
fn person() -> PersonRecord {
    PersonRecord {
        entity_id: person_id("17"),
        provider_ref: "17".into(),
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
fn person_life_and_observation_are_consistent() {
    person().validate(&tags()).unwrap();
    let mut empty_life = person();
    empty_life.death_year = Some(1300);
    empty_life.validate(&tags()).unwrap();
    let mut before_birth = person();
    before_birth.death_year = Some(1299);
    assert!(before_birth.validate(&tags()).is_err());
    for (start_year, end_year) in [(1400, 1300), (-100_001, 1300), (1300, 100_001)] {
        let mut p = person();
        p.observed_span = TimeSpan {
            start_year,
            end_year,
        };
        assert!(p.validate(&tags()).is_err(), "{start_year}/{end_year}");
    }
}
#[test]
fn person_identity_is_proved_against_its_address() {
    let mut readable = person();
    readable.entity_id = "p".into();
    let mut other_kind = person();
    other_kind.entity_id = guild("a").encode();
    let mut other_local = person();
    other_local.provider_ref = "18".into();
    for invalid in [readable, other_kind, other_local] {
        assert_eq!(
            invalid.validate(&tags()).unwrap_err().code,
            ErrorCode::InvalidRecordIdentity
        );
    }
}
#[test]
fn parent_refs_are_table_rows_or_typed_bounded_external_refs() {
    let mut p = person();
    p.parent_refs = vec![
        ParentRef::Slot {
            person_id: person_id("1"),
        },
        ParentRef::External {
            origin: ExternalParentOrigin::ExternalArrival,
            ref_id: "arrival/1".into(),
        },
    ];
    p.validate(&tags()).unwrap();
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
        person_id: person_id("3"),
    });
    let mut twice = p.clone();
    twice.parent_refs[1] = twice.parent_refs[0].clone();
    let mut too_old = p.clone();
    too_old.birth_year = -100_001;
    for invalid in [unbounded, three, twice, too_old] {
        let e = invalid.validate(&tags()).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidCanonicalValue);
        assert_eq!(e.owner, Some(person_id("17")));
    }
    let mut not_a_person = p;
    not_a_person.parent_refs[0] = ParentRef::Slot {
        person_id: "row/1".into(),
    };
    assert_eq!(
        not_a_person.validate(&tags()).unwrap_err().code,
        ErrorCode::InvalidRecordIdentity
    );
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
    // Seul le plan change de graine mondiale, références intactes : `seed` reste
    // cohérent avec les références, seule la comparaison explicite le refuse.
    let mut q = p.clone();
    q.world_seed = "43".into();
    invalid_plan(&q, &good);
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
    // Mêmes sites sous un autre ordinal : la partition attendue le refuse, et
    // elle seule, car ce plan est par ailleurs entièrement cohérent.
    let mut q = p.clone();
    with_plan_identity(&mut q, &other);
    invalid_plan(&q, &good);
    let mut moved = good.clone();
    moved.domain_sites =
        BTreeMap::from([(q.domain_id.clone(), good.domain_sites[&p.domain_id].clone())]);
    validate_plan(&q, &moved).unwrap();
    // Domaine connu mais sites différents, dans un sens ou dans l'autre.
    let mut fewer = good.clone();
    fewer
        .domain_sites
        .get_mut(&p.domain_id)
        .unwrap()
        .remove("site/demo/b");
    invalid_plan(&p, &fewer);
    let mut more = good.clone();
    more.domain_sites
        .get_mut(&p.domain_id)
        .unwrap()
        .insert("site/demo/c".into());
    invalid_plan(&p, &more);
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
/// Toutes les listes triées du plan portent au moins deux éléments distincts.
fn plan_with_full_lists() -> PopulationPlan {
    let mut p = example();
    let profiles = &mut p.resolved_profiles;
    let mut ancestry = profiles.ancestries[0].clone();
    ancestry.id = "zz-ancestry".into();
    profiles.ancestries.push(ancestry);
    let mut external = profiles.external[0].clone();
    external.id = "zz-external".into();
    profiles.external.push(external);
    let mut kinship = profiles.kinship[0].clone();
    kinship.id = "zz-kinship".into();
    profiles.kinship.push(kinship);
    let mut shock = p.shocks[0].clone();
    shock.source_event_id = "event/demo/zz".into();
    p.shocks.push(shock);
    p.shocks[0].site_ids = vec!["site/demo/a".into(), "site/demo/b".into()];
    let mut move_op = p.historical_program[0].clone();
    if let HistoricalOperation::MoveHouseholds { op_id, .. } = &mut move_op {
        *op_id = "op/demo/zz".into();
    }
    p.historical_program.push(move_op);
    p.historical_program.push(
        serde_json::from_value(json!({
            "op_id": "op/admit", "source_event_id": "event/demo/plague", "kind": "admit_people",
            "person_ids": ["person/a", "person/b"], "source_port_id": "port/demo/north",
            "year": 1370
        }))
        .unwrap(),
    );
    let mut role = p.role_demands[0].clone();
    role.slot_id = "slot/demo/zz".into();
    role.activity_ids = vec!["craft".into(), "trade".into()];
    p.role_demands.push(role);
    for id in ["person/a", "person/b"] {
        p.fixed_people.push(
            serde_json::from_value(json!({
                "person_id": id, "birth_year": 1320, "death_year": null,
                "ancestry_profile_id": "human-example", "sex": "F",
                "parent_ids": ["parent/a", "parent/b"], "initial_site_id": "site/demo/a",
                "protected_spans": []
            }))
            .unwrap(),
        );
    }
    for id in ["constraint/a", "constraint/b"] {
        p.exact_constraints.push(
            serde_json::from_value(json!({
                "constraint_id": id, "source_event_id": "event/demo/plague",
                "kind": "population_at_date", "year": 1354,
                "site_ids": ["site/demo/a", "site/demo/b"], "count": 100, "phase": 80,
                "effect_mode": "assert", "count_basis": "state_at_date"
            }))
            .unwrap(),
        );
    }
    let mut port = p.boundary_ports[0].clone();
    port.port_id = "port/demo/zz".into();
    p.boundary_ports.push(port);
    let mut edge = p.mobility_edges[0].clone();
    edge.edge_id = "edge/demo/zz".into();
    p.mobility_edges.push(edge);
    p
}
#[test]
fn plan_canonical_bytes_do_not_depend_on_any_sorted_list_order() {
    let p = plan_with_full_lists();
    let reference = plan_canonical_bytes(&p).unwrap();
    let permutations: [Permutation<PopulationPlan>; 21] = [
        ("capacités", |p| p.provider.capabilities.reverse()),
        ("ascendances", |p| p.resolved_profiles.ancestries.reverse()),
        ("classes", |p| p.resolved_profiles.social_classes.reverse()),
        ("profils extérieurs", |p| {
            p.resolved_profiles.external.reverse()
        }),
        ("parentés", |p| p.resolved_profiles.kinship.reverse()),
        ("sites", |p| p.settlements.reverse()),
        ("marqueurs", |p| p.baseline_markers.reverse()),
        ("chocs", |p| p.shocks.reverse()),
        ("sites d'un choc", |p| p.shocks[0].site_ids.reverse()),
        ("opérations", |p| p.historical_program.reverse()),
        ("personnes admises", |p| {
            for op in &mut p.historical_program {
                if let HistoricalOperation::AdmitPeople { person_ids, .. } = op {
                    person_ids.reverse();
                }
            }
        }),
        ("rôles", |p| p.role_demands.reverse()),
        ("statuts d'un rôle", |p| {
            p.role_demands[0].status_ids.reverse()
        }),
        ("activités d'un rôle", |p| {
            p.role_demands[1].activity_ids.reverse()
        }),
        ("personnes fixées", |p| p.fixed_people.reverse()),
        ("parents d'une personne", |p| {
            p.fixed_people[0].parent_ids.reverse()
        }),
        ("contraintes", |p| p.exact_constraints.reverse()),
        ("sites d'une contrainte", |p| {
            p.exact_constraints[0].site_ids.reverse()
        }),
        ("ports", |p| p.boundary_ports.reverse()),
        ("routes", |p| p.mobility_edges.reverse()),
        ("tout à la fois", |p| {
            p.settlements.reverse();
            p.boundary_ports.reverse();
            p.resolved_profiles.ancestries.reverse();
        }),
    ];
    for (name, permute) in permutations {
        let mut q = p.clone();
        permute(&mut q);
        assert_ne!(q, p, "{name} : liste à un seul élément");
        assert_eq!(plan_canonical_bytes(&q).unwrap(), reference, "{name}");
    }
}
#[test]
fn plan_canonical_bytes_exclude_identity_fields_and_keep_ordered_lists() {
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
fn historical_program_is_ordered_by_year_phase_then_op_id() {
    let mut p = example();
    let op = |json: Value| serde_json::from_value::<HistoricalOperation>(json).unwrap();
    let moves = |id: &str, year: i32| {
        op(json!({
            "op_id": id, "source_event_id": "event/demo/plague", "kind": "move_households",
            "year": year, "origin_site_id": "site/demo/a", "destination_site_id": "site/demo/b",
            "destination_port_id": null, "target_households": 3, "precision": "expected"
        }))
    };
    let death = op(json!({
        "op_id": "op/z-death", "source_event_id": "event/demo/plague", "kind": "force_death",
        "person_id": "p", "year": 1350
    }));
    let protect = op(json!({
        "op_id": "op/z-protect", "source_event_id": "event/demo/plague", "kind": "protect_person",
        "person_id": "p", "valid": {"start_year": 1350, "end_year": 1352}
    }));
    let admit = op(json!({
        "op_id": "op/a-admit", "source_event_id": "event/demo/plague", "kind": "admit_people",
        "person_ids": ["p"], "source_port_id": "port/demo/north", "year": 1350
    }));
    assert_eq!(death.application_key().1, EventPhase::Deaths);
    assert_eq!(
        moves("op", 1350).application_key().1,
        EventPhase::Residences
    );
    assert_eq!(admit.application_key().1, EventPhase::Residences);
    assert_eq!(
        protect.application_key(),
        (1350, EventPhase::ContextAndShocks, "op/z-protect")
    );
    p.historical_program = vec![
        moves("op/c", 1350),
        moves("op/a", 1360),
        admit,
        death,
        moves("op/b", 1350),
        protect,
    ];
    let value: Value = serde_json::from_slice(&plan_canonical_bytes(&p).unwrap()).unwrap();
    let ids: Vec<_> = value["historical_program"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op["op_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "op/z-protect",
            "op/z-death",
            "op/a-admit",
            "op/b",
            "op/c",
            "op/a"
        ]
    );
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
        owner_scope: OwnerScope::cell("region"),
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
fn contact_owner_is_the_world_or_an_enclosing_cell() {
    let ends = [
        contact_site("site/00", "outside/0"),
        ContactEnd::External {
            context_id: "outside/0".into(),
        },
    ];
    let with_owner = |owner_scope| {
        let mut a = atlas_with_contact(ends.clone());
        a.external_contacts[0].owner_scope = owner_scope;
        a
    };
    with_owner(OwnerScope::World).validate().unwrap();
    with_owner(OwnerScope::cell("region/north"))
        .validate()
        .unwrap();
    for invalid in [
        OwnerScope::cell("region/south"),
        OwnerScope::cell("absente"),
        OwnerScope::site("site/00"),
        Owner::domain(&domain_key()).scope().clone(),
    ] {
        assert!(with_owner(invalid).validate().is_err());
    }
}
fn atlas_with_full_lists() -> HistoryAtlas {
    let mut a = atlas_with_contact([
        contact_site("site/00", "outside/0"),
        ContactEnd::External {
            context_id: "outside/0".into(),
        },
    ]);
    let mut contact = a.external_contacts[0].clone();
    contact.contact_id = "contact/1".into();
    a.external_contacts.push(contact);
    a.region_ids.push("region/north".into());
    a.cells[1].neighbors.push("region".into());
    for id in ["edge/0", "edge/1"] {
        a.mobility_edges.push(MobilityEdge {
            edge_id: id.into(),
            from_site_id: "site/00".into(),
            to_site_id: "site/01".into(),
            valid: PERIOD,
            travel_days: 1,
            bidirectional: true,
            mode: MobilityMode::Land,
        });
    }
    a
}
#[test]
fn atlas_canonical_bytes_do_not_depend_on_any_sorted_list_order() {
    let a = atlas_with_full_lists();
    let reference = a.canonical_bytes().unwrap();
    let permutations: [Permutation<HistoryAtlas>; 7] = [
        ("cellules", |a| a.cells.reverse()),
        ("régions", |a| a.region_ids.reverse()),
        ("voisins", |a| a.cells[1].neighbors.reverse()),
        ("sites d'une cellule", |a| {
            a.cells[0].settlement_summary.site_ids.reverse()
        }),
        ("sites", |a| a.settlements.reverse()),
        ("routes", |a| a.mobility_edges.reverse()),
        ("contacts", |a| a.external_contacts.reverse()),
    ];
    for (name, permute) in permutations {
        let mut b = a.clone();
        permute(&mut b);
        assert_ne!(b, a, "{name} : liste à un seul élément");
        assert_eq!(b.canonical_bytes().unwrap(), reference, "{name}");
    }
    let value: Value = serde_json::from_slice(&reference).unwrap();
    assert_eq!(value["cells"][0]["cell_id"], "region");
    assert_eq!(value["settlements"][11]["site_id"], "site/11");
    // Un fait change les octets ; une liste ordonnée n'est pas retriée.
    let mut b = a.clone();
    b.settlements[0].initial_population += 1;
    assert_ne!(b.canonical_bytes().unwrap(), reference);
    let site = &mut b.settlements[1];
    let mut later = site.occupation_episodes[0].clone();
    site.occupation_episodes[0].valid.end_year = 1350;
    later.valid.start_year = 1350;
    later.capacity += 1;
    site.occupation_episodes.push(later);
    let ordered = b.canonical_bytes().unwrap();
    b.settlements[1].occupation_episodes.reverse();
    assert_ne!(b.canonical_bytes().unwrap(), ordered);
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
fn entity_state(id: &str, at_year: i32) -> EntityState {
    EntityState {
        entity_id: id.into(),
        at_year,
        lifecycle: Lifecycle::Active,
        member_count: CountMeasure {
            value: None,
            unit: "person".into(),
            precision: Precision {
                mode: PrecisionMode::Expected,
            },
            coverage: Coverage::Partial,
            basis: "fixture".into(),
            at_year,
        },
        cohesion_bps: 6000,
        legitimacy_bps: 5000,
        wealth_index: 0,
        capabilities: vec![],
        alignment: axes(-7000, -5000),
        conduct_tags: BTreeSet::from(["violence".into(), "extortion".into()]),
        visibility: Visibility::Covert,
        goals: vec![],
        assets: vec![],
        footprint_refs: vec![],
        institution_version: "1".into(),
    }
}
#[test]
fn entity_state_stores_axes_and_tags_but_never_the_conduct_class() {
    let state = entity_state("e", 1354);
    state.validate(&tags()).unwrap();
    let v = serde_json::to_value(&state).unwrap();
    assert!(v.get("conduct_class").is_none());
    assert_eq!(v["visibility"], "covert");
    // Tags triés par ID, comme toute liste canonique.
    assert_eq!(v["conduct_tags"], json!(["extortion", "violence"]));
    assert_eq!(serde_json::from_value::<EntityState>(v).unwrap(), state);
}
#[test]
fn conduct_tags_are_ids_validated_against_the_received_catalogue() {
    // Un tag n'est pas figé dans le noyau : le catalogue reçu fait foi.
    let state = entity_state("e", 1354);
    let without_violence: BTreeSet<_> = ["extortion".to_string()].into();
    assert_eq!(
        state.validate(&without_violence).unwrap_err().code,
        ErrorCode::InvalidAlignment
    );
    let with_new_tag: BTreeSet<_> = ["extortion", "violence", "contrebande"]
        .into_iter()
        .map(String::from)
        .collect();
    let mut extended = state;
    extended.conduct_tags.insert("contrebande".into());
    extended.validate(&with_new_tag).unwrap();
    assert!(extended.validate(&tags()).is_err());
    let mut p = person();
    let mut episode = alignment_episode(1300, 1340, None);
    episode.conduct_tags.insert("contrebande".into());
    p.alignment_episodes = vec![episode];
    assert_eq!(
        p.validate(&tags()).unwrap_err().code,
        ErrorCode::InvalidAlignment
    );
    p.validate(&with_new_tag).unwrap();
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
    p.validate(&tags()).unwrap();
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(serde_json::from_value::<PersonRecord>(v).unwrap(), p);
    let mut uncaused = p.clone();
    uncaused.alignment_episodes[1].cause_event_id = None;
    let mut overlapping = p;
    overlapping.alignment_episodes[1].valid.start_year = 1339;
    for invalid in [uncaused, overlapping] {
        assert_eq!(
            invalid.validate(&tags()).unwrap_err().code,
            ErrorCode::InvalidAlignment
        );
    }
}
const OBSERVED_START: i32 = 1300;
const WORLD_END: i32 = 1400;
fn legal_event(kind: &str, ordinal: u32, year: i32) -> HistoryEvent {
    let address = event_address("region", kind, 0, ordinal);
    fact(
        &address,
        kind,
        year,
        EventPhase::InstitutionsAndVacancies,
        vec![],
    )
}
fn standing(
    subject: &str,
    jurisdiction: &str,
    status: LegalStatus,
    (start_year, end_year): (i32, i32),
    cause: Option<&HistoryEvent>,
    end_cause: Option<&HistoryEvent>,
) -> LegalStanding {
    let (subject, jurisdiction) = (guild_ref(subject), guild_ref(jurisdiction));
    let id = SemanticAddress::standing(&region_owner(), &subject, &jurisdiction, start_year, 0);
    LegalStanding {
        id: id.unwrap().encode(),
        subject,
        jurisdiction,
        status,
        valid: TimeSpan {
            start_year,
            end_year,
        },
        cause_event_id: cause.map(|e| e.id.clone()),
        end_cause_event_id: end_cause.map(|e| e.id.clone()),
        provenance: provenance(),
        rule_id: "s0-test-1".into(),
    }
}
fn check_standings(
    standings: &[LegalStanding],
    events: &[HistoryEvent],
) -> Result<(), HistoryError> {
    validate_legal_standings(standings, events, OBSERVED_START, WORLD_END)
}
fn refused_standings(standings: &[LegalStanding], events: &[HistoryEvent], message: &str) {
    assert_eq!(
        check_standings(standings, events).unwrap_err().code,
        ErrorCode::InvalidLegalStanding,
        "{message}"
    );
}
#[test]
fn legal_standing_is_relative_to_jurisdiction() {
    use LegalStatus::*;
    let edict = legal_event("edict", 0, 1350);
    let standings = [
        standing("cult", "city-a", Licit, (1300, 1350), None, None),
        standing(
            "cult",
            "city-a",
            Proscribed,
            (1350, 1400),
            Some(&edict),
            None,
        ),
        standing("cult", "city-b", Tolerated, (1300, 1400), None, None),
    ];
    check_standings(&standings, std::slice::from_ref(&edict)).unwrap();
    let at = |jurisdiction: &str, year| {
        legal_status_at(
            &standings,
            &guild_ref("cult"),
            &guild_ref(jurisdiction),
            year,
        )
    };
    // Même sujet, même date, deux statuts ; intervalle semi-ouvert.
    assert_eq!(at("city-a", 1349), Licit);
    assert_eq!(at("city-a", 1350), Proscribed);
    assert_eq!(at("city-b", 1350), Tolerated);
    // Aucune fiche dans une juridiction, ou hors de la période : licite.
    assert_eq!(at("city-c", 1350), Licit);
    assert_eq!(at("city-a", 1400), Licit);
    // Sujets légaux admis, juridiction qui gouverne seulement.
    let rebuilt = |subject: ObjectRef, jurisdiction: ObjectRef| {
        let mut s = standings[2].clone();
        let id = SemanticAddress::standing(&region_owner(), &subject, &jurisdiction, 1300, 0);
        s.id = id.unwrap().encode();
        s.subject = subject;
        s.jurisdiction = jurisdiction;
        s
    };
    let usury = SemanticAddress::activity(WORLD, "usury").unwrap().encode();
    let activity = ObjectRef::Activity(usury);
    let site = SemanticAddress::site(WORLD, "site/00").unwrap().encode();
    check_standings(&[rebuilt(activity.clone(), guild_ref("city"))], &[]).unwrap();
    let slot = ObjectRef::PersonSlot(person_id("1"));
    check_standings(&[rebuilt(slot, guild_ref("city"))], &[]).unwrap();
    for invalid in [
        rebuilt(ObjectRef::Site(site), guild_ref("city")),
        rebuilt(guild_ref("cult"), activity),
    ] {
        refused_standings(&[invalid], &[], "type interdit");
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
    for field in ["cause_event_id", "end_cause_event_id"] {
        let mut missing = v.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<LegalStanding>(missing).is_err());
    }
}
#[test]
fn legal_standing_identity_is_proved_against_its_address() {
    use LegalStatus::*;
    let good = standing("cult", "city", Licit, (1300, 1400), None, None);
    good.validate().unwrap();
    let mut readable = good.clone();
    readable.id = "a".into();
    let mut other_kind = good.clone();
    other_kind.id = guild("cult").encode();
    let mut subject = good.clone();
    subject.subject = guild_ref("autre");
    let mut jurisdiction = good.clone();
    jurisdiction.jurisdiction = guild_ref("autre");
    let mut start = good.clone();
    start.valid.start_year = 1301;
    for invalid in [readable, other_kind, subject, jurisdiction, start] {
        assert_eq!(
            invalid.validate().unwrap_err().code,
            ErrorCode::InvalidRecordIdentity
        );
        assert_eq!(
            check_standings(&[invalid], &[]).unwrap_err().code,
            ErrorCode::InvalidRecordIdentity
        );
    }
}
#[test]
fn standing_change_cites_an_event() {
    use LegalStatus::*;
    for kind in LEGAL_CHANGE_EVENT_KINDS {
        let cause = legal_event(kind, 0, 1350);
        let chain = [
            standing("g", "j", Licit, (1300, 1350), None, None),
            standing("g", "j", Proscribed, (1350, 1400), Some(&cause), None),
        ];
        check_standings(&chain, std::slice::from_ref(&cause)).unwrap();
    }
    let edict = legal_event("edict", 0, 1350);
    let late = legal_event("charter", 1, 1351);
    let war = legal_event("war", 2, 1350);
    let absent = legal_event("edict", 9, 1350);
    let events = [edict.clone(), late.clone(), war.clone()];
    let initial = standing("g", "j", Licit, (1300, 1350), None, None);
    for (cause, message) in [
        (None, "sans événement"),
        (Some(&absent), "événement absent"),
        (Some(&late), "événement postérieur"),
        (Some(&war), "autre type d'événement"),
    ] {
        let change = standing("g", "j", Proscribed, (1350, 1400), cause, None);
        refused_standings(&[change, initial.clone()], &events, message);
    }
    // Deux statuts simultanés dans la même juridiction.
    let overlap = standing("g", "j", Proscribed, (1349, 1400), Some(&edict), None);
    let whole = standing("g", "j", Licit, (1300, 1349), None, None);
    let mut overlapping = whole;
    overlapping.valid.end_year = 1350;
    refused_standings(&[overlapping, overlap], &events, "chevauchement");
    // Une autre juridiction ouvre sa propre chaîne, avec son état initial.
    let elsewhere = standing("g", "k", Proscribed, (1300, 1400), None, None);
    let whole = standing("g", "j", Licit, (1300, 1400), None, None);
    check_standings(&[whole, elsewhere], &events).unwrap();
}
#[test]
fn only_a_segment_starting_at_observed_start_may_have_no_cause() {
    use LegalStatus::*;
    let edict = legal_event("edict", 0, 1350);
    let repeal = legal_event("charter", 1, 1360);
    let events = [edict.clone(), repeal.clone()];
    // Cas de la relecture : proscrit en 1350, sans cause, licite à nouveau en 1360.
    refused_standings(
        &[standing("g", "j", Proscribed, (1350, 1360), None, None)],
        &events,
        "premier segment tardif sans cause",
    );
    refused_standings(
        &[standing("g", "j", Proscribed, (1350, 1400), None, None)],
        &events,
        "premier segment tardif sans cause, jusqu'à la fin du monde",
    );
    check_standings(
        &[standing(
            "g",
            "j",
            Proscribed,
            (1350, 1400),
            Some(&edict),
            None,
        )],
        &events,
    )
    .unwrap();
    // Hors de la période observée.
    refused_standings(
        &[standing("g", "j", Licit, (1299, 1400), None, None)],
        &events,
        "début avant l'observation",
    );
    refused_standings(
        &[standing("g", "j", Licit, (1300, 1401), None, None)],
        &events,
        "fin après le monde",
    );
}
#[test]
fn a_segment_ending_early_or_before_a_gap_cites_an_end_event() {
    use LegalStatus::*;
    let edict = legal_event("edict", 0, 1350);
    let repeal = legal_event("charter", 1, 1360);
    let later = legal_event("charter", 2, 1361);
    let trial = legal_event("heresy_trial", 3, 1370);
    let war = legal_event("war", 4, 1360);
    let events = [
        edict.clone(),
        repeal.clone(),
        later.clone(),
        trial.clone(),
        war.clone(),
    ];
    // Fin avant celle du monde, sans successeur.
    refused_standings(
        &[standing(
            "g",
            "j",
            Proscribed,
            (1350, 1360),
            Some(&edict),
            None,
        )],
        &events,
        "fin sans événement",
    );
    refused_standings(
        &[standing("g", "j", Licit, (1300, 1360), None, None)],
        &events,
        "fin sans événement depuis l'état initial",
    );
    check_standings(
        &[standing(
            "g",
            "j",
            Proscribed,
            (1350, 1360),
            Some(&edict),
            Some(&repeal),
        )],
        &events,
    )
    .unwrap();
    // Trou entre segments.
    let after_gap = standing("g", "j", Tolerated, (1370, 1400), Some(&trial), None);
    refused_standings(
        &[
            standing("g", "j", Licit, (1300, 1360), None, None),
            after_gap.clone(),
        ],
        &events,
        "trou sans événement de fin",
    );
    check_standings(
        &[
            standing("g", "j", Licit, (1300, 1360), None, Some(&repeal)),
            after_gap,
        ],
        &events,
    )
    .unwrap();
    // Un événement de fin obéit aux mêmes règles qu'une cause.
    let absent = legal_event("charter", 9, 1360);
    for (end, message) in [
        (&absent, "événement de fin absent"),
        (&later, "événement de fin postérieur"),
        (&war, "événement de fin d'un autre type"),
    ] {
        refused_standings(
            &[standing("g", "j", Licit, (1300, 1360), None, Some(end))],
            &events,
            message,
        );
    }
}
fn social_entity(
    address: &SemanticAddress,
    kind: &str,
    archetype: &str,
    nature: EntityNature,
    owner_scope: OwnerScope,
) -> SocialEntity {
    SocialEntity {
        id: address.encode(),
        semantic_address: address.clone(),
        kind: kind.into(),
        archetype_id: archetype.into(),
        nature,
        agency: Agency::Governed,
        owner_scope,
        existence: PERIOD,
        culture_refs: vec![],
        institution_ref: None,
        provenance: provenance(),
        name_tree: None,
    }
}
fn guild_entity(slot: &str) -> SocialEntity {
    social_entity(
        &guild(slot),
        "guild",
        "base_association",
        EntityNature::OrganizedGroup,
        OwnerScope::cell("region"),
    )
}
#[test]
fn social_entity_identity_is_proved_against_its_address() {
    let refused = |e: &SocialEntity| {
        assert_eq!(
            e.validate().unwrap_err().code,
            ErrorCode::InvalidRecordIdentity,
            "{e:?}"
        );
    };
    let good = guild_entity("marchands");
    good.validate().unwrap();
    // Une guilde qui porte une adresse d'événement : le cas de la relecture.
    let war = event_address("region", "war", 1350, 0);
    refused(&social_entity(
        &war,
        "guild",
        "base_association",
        EntityNature::OrganizedGroup,
        OwnerScope::cell("region"),
    ));
    let mut readable = good.clone();
    readable.id = "entity/0".into();
    refused(&readable);
    let mut other_id = good.clone();
    other_id.id = guild("autre").encode();
    refused(&other_id);
    let mut other_kind = good.clone();
    other_kind.kind = "faction".into();
    refused(&other_kind);
    let mut other_owner = good.clone();
    other_owner.owner_scope = OwnerScope::cell("region/north");
    refused(&other_owner);
    let mut person_nature = good.clone();
    person_nature.nature = EntityNature::Person;
    refused(&person_nature);
    // Slot de groupe : son archétype est dans l'adresse.
    let owner = Owner::domain(&domain_key());
    let slot = SemanticAddress::group(&owner, "base_farm", 1300, 0).unwrap();
    let nature = EntityNature::OrganizedGroup;
    let group = social_entity(&slot, "farm", "base_farm", nature, owner.scope().clone());
    group.validate().unwrap();
    let mut other_archetype = group.clone();
    other_archetype.archetype_id = "base_workshop".into();
    refused(&other_archetype);
    // Personne et ménage appartiennent à leur domaine.
    let person_address = SemanticAddress::decode(&person_id("17")).unwrap();
    let scope = owner.scope().clone();
    let person = social_entity(
        &person_address,
        "person",
        "person",
        EntityNature::Person,
        scope,
    );
    person.validate().unwrap();
    let mut group_nature = person.clone();
    group_nature.nature = EntityNature::OrganizedGroup;
    refused(&group_nature);
    let mut not_person = person;
    not_person.kind = "guild".into();
    refused(&not_person);
    let household = SemanticAddress::household(&domain_key(), "singleton/1", 1300).unwrap();
    let scope = owner.scope().clone();
    let nature = EntityNature::Community;
    let household = social_entity(&household, "household", "household_view", nature, scope);
    household.validate().unwrap();
    let mut not_household = household;
    not_household.kind = "guild".into();
    refused(&not_household);
}
#[test]
fn history_event_identity_is_proved_against_its_address() {
    let good = event("war", 1350, EventPhase::ContextAndShocks, &[]);
    good.validate().unwrap();
    let mut other_kind = good.clone();
    other_kind.kind = "plague".into();
    let mut other_owner = good.clone();
    other_owner.owner_scope = OwnerScope::cell("region/north");
    let mut readable = good.clone();
    readable.id = "event/demo/plague".into();
    let mut not_event = good.clone();
    not_event.id = guild("a").encode();
    for invalid in [other_kind, other_owner, readable, not_event] {
        assert_eq!(
            invalid.validate().unwrap_err().code,
            ErrorCode::InvalidRecordIdentity
        );
        assert_eq!(
            causal_ranks(&[invalid]).unwrap_err().code,
            ErrorCode::InvalidRecordIdentity
        );
    }
}
fn assert_order_free<T: CanonicalRecord + std::fmt::Debug + PartialEq>(
    mut items: Vec<T>,
    shuffles: &[fn(&mut T)],
) {
    let reference = canonical_list_bytes(&items).unwrap();
    for shuffle in shuffles {
        let mut changed = items.clone();
        shuffle(&mut changed[0]);
        assert_ne!(changed, items, "liste interne à un seul élément");
        assert_eq!(canonical_list_bytes(&changed).unwrap(), reference);
    }
    items.reverse();
    assert_eq!(canonical_list_bytes(&items).unwrap(), reference);
}
#[test]
fn social_lists_have_canonical_bytes_independent_of_their_order() {
    let mut a = guild_entity("a");
    a.culture_refs = vec!["culture/a".into(), "culture/b".into()];
    assert_order_free(vec![a, guild_entity("b")], &[|e| e.culture_refs.reverse()]);
    let mut state = entity_state("a", 1350);
    state.capabilities = vec!["a".into(), "b".into()];
    state.goals = vec!["a".into(), "b".into()];
    state.assets = vec!["a".into(), "b".into()];
    state.footprint_refs = vec!["a".into(), "b".into()];
    assert_order_free(
        vec![state, entity_state("a", 1351), entity_state("b", 1350)],
        &[
            |s| s.capabilities.reverse(),
            |s| s.goals.reverse(),
            |s| s.assets.reverse(),
            |s| s.footprint_refs.reverse(),
        ],
    );
    assert_order_free(
        vec![
            relation("a", "b", Predicate::Trusts),
            relation("a", "c", Predicate::Trusts),
        ],
        &[],
    );
    let mut e = event("a", 1350, EventPhase::Deaths, &["b", "c"]);
    e.participants = vec![guild_ref("a"), guild_ref("b")];
    assert_order_free(
        vec![e, event("b", 1350, EventPhase::Deaths, &[])],
        &[|e| e.causes.reverse(), |e| e.participants.reverse()],
    );
    let mut p = person();
    p.parent_refs = vec![
        ParentRef::Slot {
            person_id: person_id("1"),
        },
        ParentRef::Slot {
            person_id: person_id("2"),
        },
    ];
    p.union_refs = vec!["union/a".into(), "union/b".into()];
    let mut other = person();
    other.entity_id = person_id("18");
    assert_order_free(
        vec![p, other],
        &[|p| p.parent_refs.reverse(), |p| p.union_refs.reverse()],
    );
    use LegalStatus::*;
    assert_order_free(
        vec![
            standing("a", "j", Licit, (1300, 1400), None, None),
            standing("b", "j", Licit, (1300, 1400), None, None),
        ],
        &[],
    );
}
#[test]
fn canonical_lists_are_sorted_by_id_and_keep_ordered_episodes() {
    let (a, b) = (guild_entity("a"), guild_entity("b"));
    let (first, second) = if a.id < b.id { (a, b) } else { (b, a) };
    let bytes = canonical_list_bytes(&[second.clone(), first.clone()]).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value[0]["id"], first.id);
    assert_eq!(value[1]["id"], second.id);
    // Les épisodes sont des listes ordonnées : leur ordre reste un fait.
    let episode = |site: &str, start_year, end_year| ResidenceEpisode {
        site_id: site.into(),
        valid: TimeSpan {
            start_year,
            end_year,
        },
    };
    let mut p = person();
    p.residence_episodes = vec![episode("a", 1300, 1340), episode("b", 1340, 1356)];
    let ordered = canonical_list_bytes(std::slice::from_ref(&p)).unwrap();
    p.residence_episodes.reverse();
    assert_ne!(canonical_list_bytes(&[p]).unwrap(), ordered);
}
fn entity_value() -> Value {
    serde_json::to_value(guild_entity("marchands")).unwrap()
}
#[test]
fn name_tree_is_absent_by_default_and_kept_as_opaque_canonical_json() {
    let entity: SocialEntity = serde_json::from_value(entity_value()).unwrap();
    assert_eq!(entity.name_tree, None);
    entity.validate().unwrap();
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
