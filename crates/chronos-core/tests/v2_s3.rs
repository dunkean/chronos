//! S3 pure proofs: crises reach plans once, and the host's validator trusts tables only.
use chronos_core::v2::catalog::HistoryCatalogue;
use chronos_core::v2::general::{build_canon, AuthorConstraints, GeneralCanon, WorldConfig};
use chronos_core::v2::model::{ErrorCode, TimeSpan};
use chronos_core::v2::population::{
    parse_population_result, plan_population, validate_population, HistoricalOperation,
    PopulationPlan, PopulationResult,
};
use chronos_core::v2::region::plan_region;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn canon() -> GeneralCanon {
    let config: WorldConfig = serde_json::from_str(include_str!(
        "../../../fixtures/social-v2/region-demo.world.json"
    ))
    .unwrap();
    let atlas = serde_json::from_str(include_str!(
        "../../../fixtures/social-v2/region-demo.atlas.json"
    ))
    .unwrap();
    let catalogue: HistoryCatalogue = serde_json::from_str(include_str!(
        "../../../docs/design/chronos-social-v2/culture-bindings.v1.json"
    ))
    .unwrap();
    build_canon(&config, &atlas, &catalogue, &AuthorConstraints::default()).unwrap()
}

/// World seed 11 of the demo fixture, every region and domain.
#[test]
fn crisis_is_applied_once() {
    let canon = canon();
    let mut applied: BTreeMap<(String, String, i32), u32> = BTreeMap::new();
    let mut moves: BTreeMap<String, u32> = BTreeMap::new();
    let mut local_shocks = 0;
    for region_id in canon.baselines.keys() {
        let region = plan_region(&canon, region_id).unwrap();
        local_shocks += region.shocks.len();
        let episodes: BTreeSet<_> = region.shocks.iter().map(|s| &s.episode_id).collect();
        assert_eq!(region.shocks.len(), episodes.len(), "one episode per site");
        for domain in canon.domains.iter().filter(|d| &d.region_id == region_id) {
            let plan = plan_population(&canon, &region, &domain.domain_id).unwrap();
            for shock in &plan.shocks {
                for site in &shock.site_ids {
                    assert!(domain.site_ids.contains(site), "shock outside its domain");
                    let key = (
                        shock.source_event_id.clone(),
                        site.clone(),
                        shock.valid.start_year,
                    );
                    *applied.entry(key).or_default() += 1;
                }
            }
            for op in &plan.historical_program {
                if let HistoricalOperation::MoveHouseholds { op_id, .. } = op {
                    *moves.entry(op_id.clone()).or_default() += 1;
                }
            }
        }
    }
    assert!(local_shocks > 0, "the demo must contain crises");
    assert_eq!(applied.values().sum::<u32>() as usize, local_shocks);
    assert!(applied.values().all(|count| *count == 1), "{applied:?}");
    assert!(moves.values().all(|count| *count == 1), "{moves:?}");
}

fn tiny_plan() -> PopulationPlan {
    let mut plan: PopulationPlan = serde_json::from_str(include_str!(
        "../../../docs/design/chronos-social-v2/village-population-plan.example.json"
    ))
    .unwrap();
    plan.timeline = TimeSpan {
        start_year: 1300,
        end_year: 1302,
    };
    plan.settlements.truncate(1);
    plan.settlements[0].initial_population = 1;
    plan
}
/// One founder resident for two years, complete and balanced.
fn tiny_result(plan: &PopulationPlan) -> Value {
    let site = &plan.settlements[0].site_id;
    let ancestry = &plan.resolved_profiles.ancestries[0].id;
    let span = json!({"start_year": 1300, "end_year": 1302});
    let census = |year: i32| {
        json!({"site_id": site, "year": year, "population": 1, "simulated": 0, "projected": 1,
               "births": 0, "deaths": 0, "incoming": 0, "outgoing": 0,
               "precision": "realized", "coverage": "complete"})
    };
    let count = |year: i32| {
        json!({"origin_cohort_id": "cohort/0", "site_id": site, "sex": 0,
               "source": "projected", "year": year, "count": 1})
    };
    json!({
        "format": "population-result-json-1", "result_type": "PopulationResult",
        "complete": true, "coverage": "complete", "stage": "test",
        "plan_id": plan.plan_id, "plan_fingerprint": plan.plan_fingerprint,
        "result_fingerprint": "not-checked-by-the-pure-core",
        "timeline": {"start_year": 1300, "end_year": 1302},
        "provider_manifest": {
            "algorithm_id": plan.provider.algorithm_id,
            "projection_version": plan.provider.projection_version,
            "runtime_fingerprint": plan.provider.runtime_fingerprint,
            "catalogue_fingerprints": plan.catalogue_fingerprints,
        },
        "candidate_table": {
            "origin_id": ["slot/0"], "origin_cohort_id": ["cohort/0"], "sex_ordinal": [0],
            "source": ["projected"], "ancestry_id": [ancestry], "sex": [0],
            "birth_year": [1270], "death_year": [null], "death_site_id": [null],
            "coverage_start_year": [1300], "coverage_end_year": [1302],
            "departure_state": ["resident_at_coverage_end"], "departure_year": [null],
            "mother_slot": [null], "father_slot": [null], "family_key": ["family/0"],
            "reproduction_flags": [0], "core_attributes": [null],
        },
        "origin_cohorts": [{"origin_cohort_id": "cohort/0", "origin_type": "founder",
            "ancestry_id": ancestry, "birth_year": 1270, "sealed": true, "sex_counts": [1, 0]}],
        "residence_index": [{"person_slot": 0, "site_id": site, "valid": span,
            "start_phase": 10, "end_phase": 0, "end_reason": "coverage_end"}],
        "residence_offsets": [0, 1],
        "household_episodes": [{"household_id": "household/0", "kind": "single",
            "formation_reason": "independent_adult", "site_id": site, "member_slots": [0],
            "valid": span, "start_phase": 10, "end_phase": 0}],
        "union_allocations": [], "birth_allocations": [], "death_allocations": [],
        "residence_transfers": [], "guardian_assignments": [], "parentage": [],
        "children_index": [[]],
        "annual_censuses": [census(1300), census(1301)],
        "cohort_residence_counts": [count(1300), count(1301)],
        "diagnostics": {}, "validation_proof": {"valid": true},
    })
}
fn parsed(value: &Value) -> PopulationResult {
    parse_population_result(&serde_json::to_vec(value).unwrap()).unwrap()
}

#[test]
fn validate_population_rejects_a_tampered_balance() {
    let plan = tiny_plan();
    let honest = tiny_result(&plan);
    let verified = validate_population(&plan, parsed(&honest)).unwrap();
    assert_eq!(verified.proof().site_years, 2);

    // The provider's proof still says valid: only the tables count.
    for (field, value) in [("population", 2), ("deaths", 1), ("incoming", 1)] {
        let mut tampered = honest.clone();
        tampered["annual_censuses"][1][field] = json!(value);
        let error = validate_population(&plan, parsed(&tampered)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidPopulationResult);
        assert_eq!(error.constraint, "population.census_differs", "{field}");
        assert_eq!(error.at_year, Some(1301));
    }
    // A resident erased from the second year cannot be balanced by any census.
    let mut erased = honest.clone();
    erased["residence_index"][0]["valid"]["end_year"] = json!(1301);
    erased["residence_index"][0]["end_reason"] = json!("death");
    erased["residence_index"][0]["end_phase"] = json!(30);
    let error = validate_population(&plan, parsed(&erased)).unwrap_err();
    assert_eq!(error.constraint, "population.residence_end_invalid");
}
