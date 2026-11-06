//! `/api/v2` routes of §14.2, served by the std HTTP server of the binary (localhost
//! only). A request whose packets are not in memory answers `202` with a job; identical
//! concurrent requests share that job. Response limits cap the response, never what is
//! generated: a job always materializes the whole domain it needs.
use super::exchange::{export_world, import_world};
use super::lineage_worker::WorkerSettings;
use super::orchestrator::{Held, Orchestrator, WorldInputs};
use super::sha256_hex;
use super::store::Store;
use chronos_core::v2::ids::SemanticAddress;
use chronos_core::v2::model::{ErrorCode, HistoryError, TimeSpan};
use chronos_core::v2::query::{
    domain_of_id, entity_sheet, explain_fact, project_view, Detail, Materialized, QueryLimits,
    QueryScope, ViewQuery,
};
use chronos_core::v2::region::RegionPlan;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub const API_PREFIX: &str = "/api/v2";
const PRESET_DIR: &str = "fixtures/social-v2";
const CATALOGUE: &str = "docs/design/chronos-social-v2/culture-bindings.v1.json";

pub struct ApiResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}
impl ApiResponse {
    fn json(status: u16, value: &Value) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec()),
        }
    }
    fn failure(status: u16, key: &str, detail: impl ToString) -> Self {
        Self::json(
            status,
            &json!({"error": {"code": "invalid_request", "key": key,
                               "params": {"detail": detail.to_string()}}}),
        )
    }
}
fn error_response(e: &HistoryError) -> ApiResponse {
    let status = match e.constraint.as_str() {
        "query.unknown_id" | "query.unknown_scope" => 404,
        "query.bad_cursor" | "query.year_out_of_range" => 400,
        "query.packet_not_materialized" => 409,
        k if k.starts_with("exchange.") => 422,
        _ => match e.code {
            ErrorCode::ProviderUnavailable | ErrorCode::ProviderVersionMismatch => 503,
            _ => 500,
        },
    };
    ApiResponse::json(
        status,
        &json!({"error": {"code": e.code, "key": e.constraint, "params": e.params,
                           "owner": e.owner, "at_year": e.at_year}}),
    )
}

pub struct WorldEntry {
    pub orch: Arc<Orchestrator>,
    pub inputs: WorldInputs,
    pub preset: String,
}
type Compute = Arc<dyn Fn(&WorldEntry) -> Result<Value, HistoryError> + Send + Sync>;

struct JobInner {
    state: &'static str,
    stage: &'static str,
    done: usize,
    total: usize,
    dependencies: Vec<Value>,
    result: Option<Value>,
    error: Option<Value>,
}
struct JobState {
    id: String,
    kind: String,
    started: Instant,
    inner: Mutex<JobInner>,
}
impl JobState {
    fn describe(&self) -> Value {
        let inner = self.inner.lock().expect("job lock");
        let mut out = json!({
            "job_id": self.id,
            "kind": self.kind,
            "state": inner.state,
            // Progress is operational, never part of a fact (§14.2).
            "progress": {"stage": inner.stage, "done": inner.done, "total": inner.total,
                          "elapsed_ms": self.started.elapsed().as_millis() as u64},
            "dependencies": inner.dependencies,
        });
        if let Some(result) = &inner.result {
            out["result"] = result.clone();
        }
        if let Some(error) = &inner.error {
            out["error"] = error.clone();
        }
        out
    }
}

pub struct ApiState {
    root: PathBuf,
    data: PathBuf,
    python: Option<PathBuf>,
    lineage_src: Option<PathBuf>,
    worlds: Mutex<BTreeMap<String, Arc<WorldEntry>>>,
    presets: Mutex<BTreeMap<String, String>>,
    jobs: Mutex<BTreeMap<String, Arc<JobState>>>,
    open_lock: Mutex<()>,
}
impl ApiState {
    pub fn new(
        root: PathBuf,
        data: PathBuf,
        python: Option<PathBuf>,
        lineage_src: Option<PathBuf>,
    ) -> Self {
        Self {
            root,
            data,
            python,
            lineage_src,
            worlds: Mutex::default(),
            presets: Mutex::default(),
            jobs: Mutex::default(),
            open_lock: Mutex::default(),
        }
    }
    fn settings(&self) -> WorkerSettings {
        WorkerSettings::resolve(
            self.python.clone(),
            self.lineage_src.clone(),
            &self.root,
            self.data.join(".cache"),
        )
    }
    fn open_inputs(&self, inputs: WorldInputs) -> Result<Orchestrator, HistoryError> {
        Orchestrator::open(inputs, self.settings(), Store::new(&self.data))
    }
    fn register(&self, orch: Orchestrator, inputs: WorldInputs, preset: &str) -> Arc<WorldEntry> {
        let canon_id = orch.canon().canon_id.clone();
        let mut worlds = self.worlds.lock().expect("worlds lock");
        worlds
            .entry(canon_id)
            .or_insert_with(|| {
                Arc::new(WorldEntry {
                    orch: Arc::new(orch),
                    inputs,
                    preset: preset.to_string(),
                })
            })
            .clone()
    }
    fn world(&self, canon_id: &str) -> Option<Arc<WorldEntry>> {
        self.worlds
            .lock()
            .expect("worlds lock")
            .get(canon_id)
            .cloned()
    }
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}
fn query_param(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == name).then(|| percent_decode(v)).flatten()
    })
}

/// Entry point: `path` is the part after `/api/v2`, `query` the raw query string.
pub fn handle(
    state: &Arc<ApiState>,
    method: &str,
    path: &str,
    query: &str,
    body: &[u8],
) -> ApiResponse {
    let Some(segments) = path
        .trim_matches('/')
        .split('/')
        .map(percent_decode)
        .collect::<Option<Vec<String>>>()
    else {
        return ApiResponse::failure(400, "api.bad_path", path);
    };
    let parts: Vec<&str> = segments.iter().map(String::as_str).collect();
    match (method, parts.as_slice()) {
        ("POST", ["worlds"]) => open_world(state, body),
        ("POST", ["import"]) => import(state, body),
        ("GET", ["jobs", id]) => job_status(state, id),
        ("GET", ["worlds", canon]) => {
            with_world(state, canon, |e| ApiResponse::json(200, &world_summary(e)))
        }
        ("POST", ["worlds", canon, "query"]) => {
            with_world(state, canon, |e| query_route(state, e, body))
        }
        ("POST", ["worlds", canon, "graph"]) => {
            with_world(state, canon, |e| graph_route(state, e, body))
        }
        ("POST", ["worlds", _, "branch"]) => ApiResponse::json(
            501,
            &json!({"error": {"code": "unsupported_capability", "key": "api.branch_unavailable",
                               "params": {"lot": "S6"}}}),
        ),
        ("GET", ["worlds", canon, "entities", id]) => {
            with_world(state, canon, |e| entity_route(state, e, id, query))
        }
        ("GET", ["worlds", canon, "explain", fact]) => {
            with_world(state, canon, |e| explain_route(state, e, fact))
        }
        ("GET", ["worlds", canon, "export"]) => with_world(state, canon, |e| {
            let with = matches!(
                query_param(query, "materialized").as_deref(),
                Some("1" | "true")
            );
            match export_world(&e.orch, &e.inputs, with) {
                Ok(bytes) => ApiResponse {
                    status: 200,
                    content_type: "application/json; charset=utf-8",
                    body: bytes,
                },
                Err(err) => error_response(&err),
            }
        }),
        (
            _,
            ["worlds"]
            | ["import"]
            | ["jobs", _]
            | ["worlds", _]
            | ["worlds", _, _]
            | ["worlds", _, _, _],
        ) => ApiResponse::failure(405, "api.method_not_allowed", method),
        _ => ApiResponse::failure(404, "api.unknown_route", path),
    }
}

fn with_world(
    state: &Arc<ApiState>,
    canon: &str,
    f: impl FnOnce(&Arc<WorldEntry>) -> ApiResponse,
) -> ApiResponse {
    match state.world(canon) {
        Some(entry) => f(&entry),
        None => ApiResponse::json(
            404,
            &json!({"error": {"code": "unknown_world", "key": "api.unknown_world",
                               "params": {"canon_id": canon}}}),
        ),
    }
}

fn world_summary(entry: &WorldEntry) -> Value {
    let canon = entry.orch.canon();
    let ready = entry.orch.ready_domains();
    let domains: Vec<Value> = canon
        .domains
        .iter()
        .map(|d| {
            json!({"domain_id": d.domain_id, "region_id": d.region_id, "site_ids": d.site_ids,
                    "population_expected_max": d.population_expected_max,
                    "materialized": ready.contains(&d.domain_id)})
        })
        .collect();
    let sites: Vec<Value> = canon
        .domains
        .iter()
        .flat_map(|d| {
            d.site_ids.iter().map(move |s| {
                json!({"site_id": s, "label": s.rsplit('/').next().unwrap_or(s),
                        "domain_id": d.domain_id})
            })
        })
        .collect();
    // The capital-sized site of the first domain: where a visitor starts.
    let default_site = canon.domains.first().and_then(|d| {
        d.site_ids
            .iter()
            .filter_map(|id| {
                let site = canon.atlas.settlements.iter().find(|s| &s.site_id == id)?;
                Some((site.population_expected_max, std::cmp::Reverse(id.clone())))
            })
            .max()
            .map(|(_, id)| id.0)
    });
    json!({
        "canon_id": canon.canon_id,
        "preset": entry.preset,
        "world_seed": canon.world_seed,
        "runtime_fingerprint": entry.orch.runtime_fingerprint(),
        "timeline": canon.timeline,
        "capabilities": {"query": true, "graph": true, "explain": true, "export": true,
                          "import": true, "overlay": true, "branch": false},
        "domains": domains,
        "sites": sites,
        "default_scope": {"site_id": default_site},
        "worker_runs": entry.orch.worker_runs(),
    })
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OpenRequest {
    preset: Option<String>,
    seed: Option<u64>,
    config: Option<Value>,
}
fn open_world(state: &Arc<ApiState>, body: &[u8]) -> ApiResponse {
    let request: OpenRequest = if body.iter().all(u8::is_ascii_whitespace) {
        OpenRequest::default()
    } else {
        match serde_json::from_slice(body) {
            Ok(r) => r,
            Err(e) => return ApiResponse::failure(400, "api.bad_request", e),
        }
    };
    let preset = request
        .preset
        .clone()
        .unwrap_or_else(|| "region-demo".into());
    if !preset
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || preset.is_empty()
    {
        return ApiResponse::failure(400, "api.bad_request", "preset");
    }
    // Pack references are names under the repository, never paths from the request.
    let key = format!("{preset}|{:?}|{}", request.seed, request.config.is_some());
    let _guard = state.open_lock.lock().expect("open lock");
    if request.config.is_none() {
        let known = state
            .presets
            .lock()
            .expect("presets lock")
            .get(&key)
            .cloned();
        if let Some(entry) = known.and_then(|c| state.world(&c)) {
            return ApiResponse::json(200, &world_summary(&entry));
        }
    }
    let dir = state.root.join(PRESET_DIR);
    let loaded = WorldInputs::load(
        &dir.join(format!("{preset}.world.json")),
        &dir.join(format!("{preset}.atlas.json")),
        &state.root.join(CATALOGUE),
    );
    let mut inputs = match loaded {
        Ok(i) => i,
        Err(e) => return error_response(&e),
    };
    if let Some(config) = request.config {
        match serde_json::from_value(config) {
            Ok(c) => inputs.config = c,
            Err(e) => return ApiResponse::failure(400, "api.bad_request", e),
        }
    }
    if let Some(seed) = request.seed {
        inputs.config.world_seed = seed;
    }
    match state.open_inputs(inputs.clone()) {
        Ok(orch) => {
            let canon_id = orch.canon().canon_id.clone();
            let entry = state.register(orch, inputs, &preset);
            state
                .presets
                .lock()
                .expect("presets lock")
                .insert(key, canon_id);
            ApiResponse::json(200, &world_summary(&entry))
        }
        Err(e) => error_response(&e),
    }
}

fn import(state: &Arc<ApiState>, body: &[u8]) -> ApiResponse {
    let _guard = state.open_lock.lock().expect("open lock");
    match import_world(body, |inputs| state.open_inputs(inputs)) {
        Ok((orch, inputs, report)) => {
            let entry = state.register(orch, inputs, "import");
            let mut summary = world_summary(&entry);
            summary["import"] = report.to_json();
            ApiResponse::json(200, &summary)
        }
        Err(e) => error_response(&e),
    }
}

fn job_status(state: &Arc<ApiState>, id: &str) -> ApiResponse {
    let job = state.jobs.lock().expect("jobs lock").get(id).cloned();
    match job {
        Some(job) => ApiResponse::json(200, &job.describe()),
        None => ApiResponse::json(
            404,
            &json!({"error": {"code": "unknown_job", "key": "api.unknown_job",
                               "params": {"job_id": id}}}),
        ),
    }
}

/// Runs inline when every packet is in memory, otherwise answers `202` with the job that
/// materializes them. A request equal to a running one joins it.
fn run_or_job(
    state: &Arc<ApiState>,
    entry: &Arc<WorldEntry>,
    kind: &str,
    request: &Value,
    needs: Vec<String>,
    compute: Compute,
) -> ApiResponse {
    if needs.iter().all(|d| entry.orch.packet_ready(d)) {
        return match compute(entry) {
            Ok(v) => ApiResponse::json(200, &v),
            Err(e) => error_response(&e),
        };
    }
    let key = format!(
        "{}|{kind}|{}",
        entry.orch.canon().canon_id,
        serde_json::to_string(request).unwrap_or_default()
    );
    let id = sha256_hex(key.as_bytes())[..24].to_string();
    let (job, created) = {
        let mut jobs = state.jobs.lock().expect("jobs lock");
        match jobs.get(&id) {
            Some(job) if job.inner.lock().expect("job lock").state != "failed" => {
                (job.clone(), false)
            }
            _ => {
                let job = Arc::new(JobState {
                    id: id.clone(),
                    kind: kind.to_string(),
                    started: Instant::now(),
                    inner: Mutex::new(JobInner {
                        state: "running",
                        stage: "queued",
                        done: 0,
                        total: needs.len() * 2 + 1,
                        dependencies: needs
                            .iter()
                            .flat_map(|d| {
                                [
                                    json!({"kind": "population", "domain_id": d}),
                                    json!({"kind": "social_packet", "domain_id": d}),
                                ]
                            })
                            .collect(),
                        result: None,
                        error: None,
                    }),
                });
                jobs.insert(id.clone(), job.clone());
                (job, true)
            }
        }
    };
    if created {
        let entry = entry.clone();
        let worker = job.clone();
        std::thread::spawn(move || {
            let step = |stage: &'static str| {
                let mut inner = worker.inner.lock().expect("job lock");
                inner.stage = stage;
            };
            let advance = || worker.inner.lock().expect("job lock").done += 1;
            let outcome = (|| {
                for domain in &needs {
                    step("population");
                    entry.orch.population(domain)?;
                    advance();
                    step("packet");
                    entry.orch.social_packet(domain)?;
                    advance();
                }
                step("projection");
                compute(&entry)
            })();
            let mut inner = worker.inner.lock().expect("job lock");
            match outcome {
                Ok(value) => {
                    inner.state = "done";
                    inner.stage = "done";
                    inner.done = inner.total;
                    inner.result = Some(value);
                }
                Err(e) => {
                    inner.state = "failed";
                    inner.error = serde_json::to_value(&e).ok().map(|v| {
                        json!({"code": v["code"], "key": e.constraint, "params": e.params,
                                "owner": e.owner})
                    });
                }
            }
        });
    }
    let mut body = job.describe();
    body["poll"] = json!(format!("{API_PREFIX}/jobs/{}", job.id));
    ApiResponse::json(202, &body)
}

fn held_for(entry: &WorldEntry, needs: &[String]) -> Result<Vec<Held>, HistoryError> {
    needs.iter().map(|d| entry.orch.held(d)).collect()
}
fn compute_query(
    entry: &WorldEntry,
    query: &ViewQuery,
    needs: &[String],
) -> Result<Value, HistoryError> {
    let held = held_for(entry, needs)?;
    // Only the domains the request names: a view never depends on what else happens to
    // be in memory.
    let mats: Vec<Materialized> = held
        .iter()
        .map(|h| Materialized {
            region: &h.region,
            population: &h.population,
            packet: &h.packet,
        })
        .collect();
    let mut view = project_view(entry.orch.canon(), &mats, query)?;
    view.view_fingerprint = sha256_hex(&view.fingerprint_bytes()?);
    serde_json::to_value(&view).map_err(|e| {
        HistoryError::new(ErrorCode::InvalidCanonicalValue, "api.encode")
            .with_param("detail", e.to_string())
    })
}
fn needs_of(entry: &WorldEntry, query: &ViewQuery) -> Result<Vec<String>, HistoryError> {
    let canon = entry.orch.canon();
    let unknown = |what: &str| HistoryError {
        owner: Some(what.into()),
        ..HistoryError::new(ErrorCode::InvalidRecordIdentity, "query.unknown_scope")
    };
    let domain = if let Some(site) = &query.scope.site_id {
        canon
            .domain_of_site(site)
            .map(|d| d.domain_id.clone())
            .ok_or_else(|| unknown(site))?
    } else if let Some(d) = &query.scope.domain_id {
        canon
            .domains
            .iter()
            .find(|x| &x.domain_id == d)
            .map(|x| x.domain_id.clone())
            .ok_or_else(|| unknown(d))?
    } else if let Some(id) = query.scope.ids.first() {
        domain_of_id(canon, id).ok_or_else(|| unknown(id))?
    } else {
        return Err(unknown("scope"));
    };
    Ok(vec![domain])
}

fn query_route(state: &Arc<ApiState>, entry: &Arc<WorldEntry>, body: &[u8]) -> ApiResponse {
    let request: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => return ApiResponse::failure(400, "api.bad_request", e),
    };
    let query: ViewQuery = match serde_json::from_value(request.clone()) {
        Ok(q) => q,
        Err(e) => return ApiResponse::failure(400, "api.bad_request", e),
    };
    let needs = match needs_of(entry, &query) {
        Ok(n) => n,
        Err(e) => return error_response(&e),
    };
    let wanted = needs.clone();
    run_or_job(
        state,
        entry,
        "query",
        &request,
        needs,
        Arc::new(move |e| compute_query(e, &query, &wanted)),
    )
}

#[derive(Deserialize)]
struct GraphRequest {
    center: String,
    at_year: Option<i32>,
    period: Option<TimeSpan>,
    depth: Option<u8>,
    include: Option<Vec<String>>,
    limits: Option<QueryLimits>,
    cursor: Option<String>,
}
fn graph_route(state: &Arc<ApiState>, entry: &Arc<WorldEntry>, body: &[u8]) -> ApiResponse {
    let request: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => return ApiResponse::failure(400, "api.bad_request", e),
    };
    let graph: GraphRequest = match serde_json::from_value(request.clone()) {
        Ok(g) => g,
        Err(e) => return ApiResponse::failure(400, "api.bad_request", e),
    };
    let query = ViewQuery {
        scope: QueryScope {
            ids: vec![graph.center],
            ..QueryScope::default()
        },
        at_year: graph.at_year,
        period: graph.period,
        include: graph
            .include
            .unwrap_or_else(|| vec!["people".into(), "events".into()]),
        detail: if graph.depth.unwrap_or(1) >= 2 {
            Detail::Extended
        } else {
            Detail::Local
        },
        limits: graph.limits.unwrap_or_default(),
        cursor: graph.cursor,
    };
    let needs = match needs_of(entry, &query) {
        Ok(n) => n,
        Err(e) => return error_response(&e),
    };
    let wanted = needs.clone();
    run_or_job(
        state,
        entry,
        "graph",
        &request,
        needs,
        Arc::new(move |e| compute_query(e, &query, &wanted)),
    )
}

fn entity_route(
    state: &Arc<ApiState>,
    entry: &Arc<WorldEntry>,
    id: &str,
    query: &str,
) -> ApiResponse {
    let canon = entry.orch.canon();
    let Some(domain) = domain_of_id(canon, id) else {
        return ApiResponse::json(
            404,
            &json!({"error": {"code": "invalid_record_identity", "key": "query.unknown_id",
                               "params": {}, "owner": id}}),
        );
    };
    let at_year: i32 = query_param(query, "at_year")
        .and_then(|y| y.parse().ok())
        .unwrap_or(canon.timeline.start_year);
    let id = id.to_string();
    let wanted = vec![domain.clone()];
    run_or_job(
        state,
        entry,
        "entity",
        &json!({"id": id, "at_year": at_year}),
        vec![domain],
        Arc::new(move |e| {
            let held = held_for(e, &wanted)?;
            let mats: Vec<Materialized> = held
                .iter()
                .map(|h| Materialized {
                    region: &h.region,
                    population: &h.population,
                    packet: &h.packet,
                })
                .collect();
            entity_sheet(e.orch.canon(), &mats, &id, at_year)
        }),
    )
}

fn explain_route(state: &Arc<ApiState>, entry: &Arc<WorldEntry>, fact: &str) -> ApiResponse {
    if SemanticAddress::decode(fact).is_err() {
        return ApiResponse::json(
            404,
            &json!({"error": {"code": "invalid_record_identity", "key": "query.unknown_id",
                               "params": {}, "owner": fact}}),
        );
    }
    let needs: Vec<String> = domain_of_id(entry.orch.canon(), fact).into_iter().collect();
    let wanted = needs.clone();
    let fact = fact.to_string();
    run_or_job(
        state,
        entry,
        "explain",
        &json!({"fact_id": fact}),
        needs,
        Arc::new(move |e| {
            let held = held_for(e, &wanted)?;
            let mats: Vec<Materialized> = held
                .iter()
                .map(|h| Materialized {
                    region: &h.region,
                    population: &h.population,
                    packet: &h.packet,
                })
                .collect();
            let regions: Vec<Arc<RegionPlan>> = e
                .orch
                .canon()
                .atlas
                .region_ids
                .iter()
                .map(|r| e.orch.plan_region(r))
                .collect::<Result<_, _>>()?;
            let refs: Vec<&RegionPlan> = regions.iter().map(|r| r.as_ref()).collect();
            explain_fact(e.orch.canon(), &refs, &mats, &fact)
        }),
    )
}
