//! S5 exit proof on the real Lineage Lab worker: `/api/v2` through the host handler,
//! pagination, shared jobs, export and import, and the seed sampler. Skips with a message
//! when the interpreter is absent; set CHRONOS_PYTHON and CHRONOS_LINEAGE_SRC to run it.
use chronos_app::v2::exchange::{export_world, import_world};
use chronos_app::v2::lineage_worker::WorkerSettings;
use chronos_app::v2::orchestrator::{Orchestrator, WorldInputs};
use chronos_app::v2::sample::{run_sample, SampleOptions};
use chronos_app::v2::server::{handle, ApiResponse, ApiState};
use chronos_app::v2::store::Store;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn worker_available() -> bool {
    let settings = WorkerSettings::resolve(None, None, &root(), PathBuf::new());
    let ok = settings.python.is_file() && settings.lineage_src.join("lineage_lab").is_dir();
    if !ok {
        eprintln!("SKIPPED S5 host: set CHRONOS_PYTHON and CHRONOS_LINEAGE_SRC");
    }
    ok
}
fn data_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
}
fn call(
    state: &Arc<ApiState>,
    method: &str,
    path: &str,
    body: &Value,
) -> (u16, Value, ApiResponse) {
    let bytes = if body.is_null() {
        Vec::new()
    } else {
        serde_json::to_vec(body).unwrap()
    };
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let response = handle(state, method, path, query, &bytes);
    let value = serde_json::from_slice(&response.body).unwrap_or(Value::Null);
    (response.status, value, response)
}
/// Polls a job to its end and returns its result.
fn finish(state: &Arc<ApiState>, job: &Value) -> Value {
    let id = job["job_id"].as_str().expect("job id");
    let started = Instant::now();
    loop {
        let (status, body, _) = call(state, "GET", &format!("/jobs/{id}"), &Value::Null);
        assert_eq!(status, 200);
        match body["state"].as_str() {
            Some("done") => return body["result"].clone(),
            Some("failed") => panic!("job failed: {body}"),
            _ => {}
        }
        assert!(
            started.elapsed() < Duration::from_secs(1500),
            "job too slow"
        );
        std::thread::sleep(Duration::from_millis(500));
    }
}
fn open(state: &Arc<ApiState>) -> Value {
    let (status, world, _) = call(
        state,
        "POST",
        "/worlds",
        &json!({"preset": "region-demo", "seed": 11}),
    );
    assert_eq!(status, 200, "{world}");
    world
}

#[test]
fn api_v2_serves_stable_paged_views_and_shares_jobs() {
    if !worker_available() {
        return;
    }
    let state = Arc::new(ApiState::new(root(), data_dir("s5-api"), None, None));
    let world = open(&state);
    let canon = world["canon_id"].as_str().unwrap().to_string();
    assert_eq!(world["capabilities"]["branch"], false);
    let (status, body, _) = call(
        &state,
        "POST",
        &format!("/worlds/{canon}/branch"),
        &json!({}),
    );
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (501, Some("unsupported_capability"))
    );

    let site = "site/demo/underhill";
    let query = json!({
        "scope": {"site_id": site}, "at_year": 1360,
        "period": {"start_year": 1335, "end_year": 1386},
        "limits": {"people": 30, "relations": 120, "groups": 20},
    });
    let path = format!("/worlds/{canon}/query");
    let (status, first, _) = call(&state, "POST", &path, &query);
    // Whatever the cache state, a second identical request must give the same thing.
    let view = if status == 202 {
        // Concurrent requests share the job.
        let (status_b, second, _) = call(&state, "POST", &path, &query);
        assert_eq!(status_b, 202);
        assert_eq!(first["job_id"], second["job_id"], "same request, same job");
        finish(&state, &first)
    } else {
        first
    };
    let (status, again, _) = call(&state, "POST", &path, &query);
    assert_eq!(status, 200, "a ready packet answers inline");
    assert_eq!(view["view_fingerprint"], again["view_fingerprint"]);
    assert_eq!(view, again);

    // Identity: entity IDs and person addresses only; names are declared fallbacks.
    let en: Value =
        serde_json::from_slice(&std::fs::read(root().join("web/i18n/en.json")).unwrap()).unwrap();
    for e in view["entities"].as_array().unwrap() {
        let id = e["id"].as_str().unwrap();
        assert!(
            id.starts_with("person.") || id.starts_with("group.") || id.starts_with("entity."),
            "{id}"
        );
        assert_eq!(e["name_synthetic"], true);
        if e["kind"] != "person" && e["kind"] != "polity" {
            assert_eq!(e["name_key"], "org_name.fallback");
            let key = e["name_params"]["kind_key"].as_str().unwrap();
            assert!(en.get(key).is_some(), "missing label {key}");
            assert!(en
                .get(format!("kind.{}", e["kind"].as_str().unwrap()))
                .is_some());
        }
    }

    // Pagination: pages are stable, disjoint and add up to the same total.
    let small = json!({
        "scope": {"site_id": site}, "at_year": 1360,
        "period": {"start_year": 1355, "end_year": 1366},
        "limits": {"people": 8, "relations": 25, "groups": 10},
    });
    let mut seen = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut cursor = Value::Null;
    let mut pages = 0;
    loop {
        let mut q = small.clone();
        if !cursor.is_null() {
            q["cursor"] = cursor.clone();
        }
        let (status, page, _) = call(&state, "POST", &path, &q);
        assert_eq!(status, 200, "{page}");
        for e in page["entities"].as_array().unwrap() {
            assert!(
                seen.insert(e["id"].as_str().unwrap().to_string()),
                "entity repeated across pages"
            );
        }
        for r in page["relations"].as_array().unwrap() {
            assert!(
                edges.insert(r["id"].as_str().unwrap().to_string()),
                "relation repeated across pages"
            );
        }
        let (_, repeat, _) = call(&state, "POST", &path, &q);
        assert_eq!(page, repeat, "a page is stable");
        pages += 1;
        cursor = page["limits"]["cursor"].clone();
        let complete = page["limits"]["complete"].as_bool().unwrap();
        assert_eq!(
            complete,
            cursor.is_null(),
            "complete means that no page follows"
        );
        assert!(pages > 1 || !complete, "a cut first page is never complete");
        if cursor.is_null() {
            break;
        }
        assert!(pages < 200);
    }
    assert!(pages > 1, "the small limits must cut the view");
    let (_, whole, _) = call(
        &state,
        "POST",
        &path,
        &json!({
            "scope": {"site_id": site}, "at_year": 1360,
            "period": {"start_year": 1355, "end_year": 1366},
            "limits": {"people": 100000, "relations": 100000, "groups": 10},
        }),
    );
    let total: BTreeSet<String> = whole["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(seen, total, "the pages add up to the unpaged view");
    assert_eq!(whole["limits"]["complete"], true);

    // The mayor of 1360 and his neighbourhood, by entity then by graph and explanation.
    let mayor_office = whole["default_selection"]["office_id"].as_str().unwrap();
    let tenure = whole["tenures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| {
            t["office_id"] == mayor_office
                && t["valid"][0].as_i64() <= Some(1360)
                && Some(1360) < t["valid"][1].as_i64()
        })
        .expect("a mayor tenure covers 1360");
    let person = tenure["person_id"].as_str().expect("a held tenure");
    let enc = |s: &str| s.replace('/', "%2F");
    let (status, sheet, _) = call(
        &state,
        "GET",
        &format!("/worlds/{canon}/entities/{}?at_year=1360", enc(person)),
        &Value::Null,
    );
    assert_eq!(status, 200, "{sheet}");
    assert_eq!(sheet["kind"], "person");
    let (status, graph, _) = call(
        &state,
        "POST",
        &format!("/worlds/{canon}/graph"),
        &json!({"center": person, "at_year": 1360, "depth": 1}),
    );
    assert_eq!(status, 200, "{graph}");
    assert!(graph["entities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["id"] == person));
    let tenure_id = tenure["tenure_id"].as_str().unwrap();
    let (status, why, _) = call(
        &state,
        "GET",
        &format!("/worlds/{canon}/explain/{tenure_id}"),
        &Value::Null,
    );
    assert_eq!(status, 200, "{why}");
    assert_eq!(why["kind"], "tenure");
    assert!(why["selection"]["candidates"].is_u64());
    // A crisis cause is reachable from a tenure that cites an event, and explained.
    let cited = whole["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "war" || e["kind"] == "epidemic");
    if let Some(crisis) = cited {
        let id = crisis["id"].as_str().unwrap();
        let (status, why, _) = call(
            &state,
            "GET",
            &format!("/worlds/{canon}/explain/{id}"),
            &Value::Null,
        );
        assert_eq!(status, 200, "{why}");
        assert_eq!(why["kind"], "event");
    }

    // The map overlay: atlas positions only, legality channel, contacts without position.
    let overlay = &whole["overlay"];
    assert!(overlay["channel_names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c == "legality"));
    assert!(!overlay["sites"].as_array().unwrap().is_empty());
    for c in overlay["external_contacts"].as_array().unwrap() {
        assert!(c.get("x_mm").is_none() && c.get("y_mm").is_none());
        for end in c["site_ends"].as_array().unwrap() {
            assert!(end.get("x_mm").is_none());
        }
    }
}

#[test]
fn export_import_gives_the_same_facts_without_rerunning_the_provider() {
    if !worker_available() {
        return;
    }
    let root = root();
    let inputs = WorldInputs::load(
        &root.join("fixtures/social-v2/region-demo.world.json"),
        &root.join("fixtures/social-v2/region-demo.atlas.json"),
        &root.join("docs/design/chronos-social-v2/culture-bindings.v1.json"),
    )
    .unwrap();
    let settings = |dir: &Path| WorkerSettings::resolve(None, None, &root, dir.join(".cache"));
    let source = data_dir("s5-export-source");
    let orchestrator =
        Orchestrator::open(inputs.clone(), settings(&source), Store::new(&source)).unwrap();
    let canon_only = export_world(&orchestrator, &inputs, false).unwrap();
    assert_eq!(
        canon_only,
        export_world(&orchestrator, &inputs, false).unwrap(),
        "export is deterministic"
    );
    assert!(
        canon_only.len() < 4_000_000,
        "a canon-only export does not depend on visited zones"
    );

    // A damaged export is refused before anything is trusted.
    let mut broken: Value = serde_json::from_slice(&canon_only).unwrap();
    broken["canon_sha256"] = json!("0".repeat(64));
    let target = data_dir("s5-export-refused");
    let refused = import_world(&serde_json::to_vec(&broken).unwrap(), |i| {
        Orchestrator::open(i, settings(&target), Store::new(&target))
    });
    assert!(refused.is_err());

    // Canon only: replayed, nothing materialized.
    let fresh = data_dir("s5-export-canon");
    let (imported, _, report) = import_world(&canon_only, |i| {
        Orchestrator::open(i, settings(&fresh), Store::new(&fresh))
    })
    .unwrap();
    assert!(report.canon_replayed && report.domains.is_empty());
    assert_eq!(imported.canon().canon_id, orchestrator.canon().canon_id);
    assert_eq!(imported.worker_runs(), 0);

    // With the computed packets: the archive is checked, the packet matches, the provider
    // is not rerun.
    let domain = &orchestrator.domain_ids()[1];
    let packet = orchestrator.social_packet(domain).unwrap();
    let full = export_world(&orchestrator, &inputs, true).unwrap();
    assert_eq!(full, export_world(&orchestrator, &inputs, true).unwrap());
    let reopened = data_dir("s5-export-full");
    let _ = std::fs::remove_dir_all(&reopened);
    let (second, _, report) = import_world(&full, |i| {
        Orchestrator::open(i, settings(&reopened), Store::new(&reopened))
    })
    .unwrap();
    assert_eq!(
        second.worker_runs(),
        0,
        "an import never starts the provider"
    );
    assert_eq!(report.domains.len(), 1);
    assert_eq!(report.domains[0]["packet_matches"], true);
    assert_eq!(report.domains[0]["provider_replayed"], false);
    let again = second.social_packet(domain).unwrap();
    assert_eq!(
        packet.canonical_bytes().unwrap(),
        again.canonical_bytes().unwrap()
    );
}

#[test]
fn sample_exports_aggregates_with_versions() {
    if !worker_available() {
        return;
    }
    let root = root();
    let out = data_dir("s5-sample");
    let options = SampleOptions {
        world: root.join("fixtures/social-v2/region-demo.world.json"),
        atlas: root.join("fixtures/social-v2/region-demo.atlas.json"),
        catalogue: root.join("docs/design/chronos-social-v2/culture-bindings.v1.json"),
        seeds: vec![11],
        years: vec![1340, 1360],
        domains: vec![1],
        settings: WorkerSettings::resolve(None, None, &root, out.join(".cache")),
        out: out.clone(),
    };
    let summary = run_sample(&options).unwrap();
    assert_eq!(summary["seeds"], json!([11]));
    assert!(summary["versions"]["runtime_fingerprint"].is_string());
    assert!(summary["distributions"]["slots"]["n"] == 1);
    assert!(
        summary["distributions"]["year_1360.population"]["max"]
            .as_u64()
            .unwrap()
            > 0
    );
    let seed: Value =
        serde_json::from_slice(&std::fs::read(out.join("seed-11.json")).unwrap()).unwrap();
    assert_eq!(seed["seed"], 11);
    assert!(seed["domains"][0]["balance_closed"] == true);
}
