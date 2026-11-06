//! S3 exit proofs on the real Lineage Lab worker. Each test skips with a message when
//! the interpreter is absent; set CHRONOS_PYTHON and CHRONOS_LINEAGE_SRC to run them.
use chronos_app::v2::lineage_worker::{LineageWorker, WorkerSettings};
use chronos_app::v2::orchestrator::{Orchestrator, WorldInputs};
use chronos_app::v2::store::Store;
use chronos_core::v2::canonical::canonical_bytes;
use chronos_core::v2::model::ErrorCode;
use chronos_core::v2::population::{validate_population, VerifiedPopulation};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
/// `None`, with the reason on stderr, when the provider cannot run here.
fn settings(test: &str) -> Option<WorkerSettings> {
    // Shared by the tests: the JIT cache never enters a fingerprint.
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lineage-cache");
    let settings = WorkerSettings::resolve(None, None, &root(), cache);
    if settings.python.is_file() && settings.lineage_src.join("lineage_lab").is_dir() {
        Some(settings)
    } else {
        eprintln!(
            "SKIPPED {test}: no interpreter at {} or no provider at {}; \
             set CHRONOS_PYTHON and CHRONOS_LINEAGE_SRC",
            settings.python.display(),
            settings.lineage_src.display()
        );
        None
    }
}
fn inputs() -> WorldInputs {
    let root = root();
    WorldInputs::load(
        &root.join("fixtures/social-v2/region-demo.world.json"),
        &root.join("fixtures/social-v2/region-demo.atlas.json"),
        &root.join("docs/design/chronos-social-v2/culture-bindings.v1.json"),
    )
    .unwrap()
}
fn fresh_store(name: &str) -> Store {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&path);
    Store::new(path)
}
fn open(settings: &WorkerSettings, store: Store) -> Orchestrator {
    Orchestrator::open(inputs(), settings.clone(), store).unwrap()
}
/// Canonical identity of a population, per domain.
type Identities = BTreeMap<String, (String, String)>;
fn identity(population: &VerifiedPopulation) -> (String, String) {
    let proof = population.proof();
    (
        proof.plan_fingerprint.clone(),
        proof.result_fingerprint.clone(),
    )
}
fn query_in_order(settings: &WorkerSettings, name: &str, order: &[usize]) -> Identities {
    let orchestrator = open(settings, fresh_store(name));
    let domains = orchestrator.domain_ids();
    order
        .iter()
        .map(|i| {
            let population = orchestrator.population(&domains[*i]).unwrap();
            (domains[*i].clone(), identity(&population))
        })
        .collect()
}

/// S3 exit proof (§20.2), demo fixture, world seed 11, two domains: forward, reversed,
/// concurrent, then reversed inside one shared worker process.
#[test]
fn same_population_for_three_query_orders() {
    let Some(settings) = settings("same_population_for_three_query_orders") else {
        return;
    };
    let forward = query_in_order(&settings, "order-forward", &[0, 1]);
    assert_eq!(forward.len(), 2, "the demo has two domains");
    let reversed = query_in_order(&settings, "order-reversed", &[1, 0]);
    assert_eq!(forward, reversed);

    let orchestrator = Arc::new(open(&settings, fresh_store("order-concurrent")));
    let handles: Vec<_> = orchestrator
        .domain_ids()
        .into_iter()
        .map(|domain| {
            let orchestrator = orchestrator.clone();
            std::thread::spawn(move || {
                let population = orchestrator.population(&domain).unwrap();
                (domain, identity(&population))
            })
        })
        .collect();
    let concurrent: Identities = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(forward, concurrent);

    // One process, both plans prepared, reversed: per-job state must not leak.
    let jobs = fresh_store("order-shared-process");
    let mut worker = LineageWorker::spawn(&settings, jobs.root()).unwrap();
    let plans: Vec<_> = orchestrator
        .domain_ids()
        .iter()
        .map(|domain| orchestrator.plan_population(domain).unwrap())
        .collect();
    plans.iter().for_each(|plan| worker.prepare(plan).unwrap());
    for plan in plans.iter().rev() {
        let artifact = worker.materialize(plan).unwrap();
        assert_eq!(forward[&plan.domain_id].1, artifact.result_fingerprint);
    }
}

fn smallest_domain(orchestrator: &Orchestrator) -> String {
    let canon = orchestrator.canon();
    let population = |id: &String| {
        canon
            .atlas
            .settlements
            .iter()
            .filter(|s| id == &s.site_id)
            .map(|s| s.initial_population)
            .sum::<u64>()
    };
    canon
        .domains
        .iter()
        .min_by_key(|d| d.site_ids.iter().map(population).sum::<u64>())
        .unwrap()
        .domain_id
        .clone()
}

/// Demo fixture, world seed 11, smallest domain. Concurrent requests share one job;
/// a later process reads the store, revalidates, and never starts the worker.
#[test]
fn cold_and_warm_population_match() {
    let Some(settings) = settings("cold_and_warm_population_match") else {
        return;
    };
    let store = fresh_store("cold-warm");
    let cold = Arc::new(open(&settings, store.clone()));
    let domain = smallest_domain(&cold);
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let (cold, domain) = (cold.clone(), domain.clone());
            std::thread::spawn(move || cold.population(&domain).unwrap())
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(cold.worker_runs(), 1, "concurrent requests share one job");
    assert!(results.iter().all(|r| Arc::ptr_eq(r, &results[0])));

    let warm = open(&settings, store.clone());
    assert_eq!(warm.canon().canon_id, cold.canon().canon_id);
    let again = warm.population(&domain).unwrap();
    assert_eq!(warm.worker_runs(), 0, "a warm read starts no provider");
    assert_eq!(*results[0], *again);

    // No job or staging residue next to the published package.
    let world = store.world_dir(&warm.canon().canon_id);
    let entries = |path: PathBuf| std::fs::read_dir(path).map_or(0, |d| d.count());
    assert_eq!(entries(world.join(".staging")), 0);
    assert_eq!(entries(store.root().join(".jobs")), 0);

    // Real tables, tampered census: the provider's own proof does not save it.
    let plan = warm.plan_population(&domain).unwrap();
    let mut tampered = again.result().clone();
    tampered.annual_censuses[7].population += 1;
    let error = validate_population(&plan, tampered).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidPopulationResult);

    // A damaged archive fails loudly and is not silently recomputed.
    let package = std::fs::read_dir(world.join("populations"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    let archive = package.path().join("provider-archive");
    let mut bytes = std::fs::read(&archive).unwrap();
    let last = bytes.len() - 2;
    bytes[last] ^= 1;
    std::fs::write(&archive, bytes).unwrap();
    let damaged = open(&settings, store);
    let error = damaged.population(&domain).unwrap_err();
    assert_eq!(error.constraint, "store.package_corrupt");
    assert_eq!(damaged.worker_runs(), 0);
}

/// Rust and the provider serialize the shared vectors to the same bytes (§4.2).
#[test]
fn canonical_json_vectors_match_the_python_provider() {
    let Some(settings) = settings("canonical_json_vectors_match_the_python_provider") else {
        return;
    };
    let path = root().join("docs/design/chronos-social-v2/vectors/canonical-json.v1.json");
    let vectors: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let script = "import json,sys\n\
        from lineage_lab.chronos_plan import canonical_bytes\n\
        vectors=json.load(open(sys.argv[1],encoding='utf-8'))['vectors']\n\
        print(json.dumps([canonical_bytes(v['input']).hex() for v in vectors]))\n";
    let output = Command::new(&settings.python)
        .args(["-c", script])
        .arg(&path)
        .env("PYTHONPATH", &settings.lineage_src)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let python: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    let cases = vectors["vectors"].as_array().unwrap();
    assert_eq!(python.len(), cases.len());
    assert!(!cases.is_empty());
    for (hex, case) in python.iter().zip(cases) {
        let rust: String = canonical_bytes(&case["input"])
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(*hex, rust, "{}", case["name"]);
    }
}
