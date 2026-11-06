//! Pins the runtime, builds the canon, plans regions and domains, and resolves a
//! domain's population exactly once (§14, §16). Concurrent requests share one job.
use super::lineage_worker::{query_runtime, run_plan, RuntimeIdentity, WorkerSettings, PROTOCOL};
use super::store::{unique_name, PopulationCacheKey, Store};
use super::{host_error, sha256_hex};
use chronos_core::v2::canonical::canonical_bytes;
use chronos_core::v2::catalog::HistoryCatalogue;
use chronos_core::v2::general::{build_canon, AuthorConstraints, GeneralCanon, WorldConfig};
use chronos_core::v2::model::{ErrorCode, HistoryError};
use chronos_core::v2::population::{
    parse_population_result, plan_canonical_bytes, plan_population, validate_population,
    PopulationPlan, VerifiedPopulation, POPULATION_RESULT_FORMAT,
};
use chronos_core::v2::region::{plan_region, RegionPlan};
use chronos_core::v2::social::{build_social_packet, SocialPacket};
use chronos_core::v2::{atlas::HistoryAtlas, ALGORITHM_VERSION};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// Value of a world file that leaves the runtime to be pinned by the host.
pub const UNPINNED_RUNTIME: &str = "synthetic-runtime-not-pinned";
const WORLD_MANIFEST_FORMAT: &str = "chronos-world-manifest-1";
/// Level of detail of a package (§16.1): the complete S3 registry.
const POPULATION_DETAIL: &str = "complete_registry";

fn input_error(key: &str, path: &Path, detail: impl ToString) -> HistoryError {
    host_error(
        ErrorCode::InvalidCanonicalValue,
        key,
        format!("{}: {}", path.display(), detail.to_string()),
    )
}
/// Frozen inputs of a world, with the exact bytes their fingerprints cover (§4.2).
#[derive(Clone, Debug)]
pub struct WorldInputs {
    pub config: WorldConfig,
    pub atlas: HistoryAtlas,
    pub catalogue: HistoryCatalogue,
    pub constraints: AuthorConstraints,
    pub atlas_bytes: Vec<u8>,
    pub catalogue_bytes: Vec<u8>,
}
fn read_json<T: DeserializeOwned>(path: &Path) -> Result<(T, Vec<u8>), HistoryError> {
    let bytes =
        std::fs::read(path).map_err(|e| input_error("orchestrator.input_unreadable", path, e))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|e| input_error("orchestrator.input_invalid", path, e))?;
    Ok((value, bytes))
}
impl WorldInputs {
    pub fn load(world: &Path, atlas: &Path, catalogue: &Path) -> Result<Self, HistoryError> {
        let (config, _) = read_json(world)?;
        let (atlas, atlas_bytes) = read_json(atlas)?;
        let (catalogue, catalogue_bytes) = read_json(catalogue)?;
        Ok(Self {
            config,
            atlas,
            catalogue,
            constraints: AuthorConstraints::default(),
            atlas_bytes,
            catalogue_bytes,
        })
    }
}
/// Normalized input manifest (§4.2): resolved configuration, runtime and pack
/// fingerprints, never anything that already contains the canon ID.
fn world_manifest(
    inputs: &WorldInputs,
    config: &WorldConfig,
    runtime: &RuntimeIdentity,
) -> Result<Value, HistoryError> {
    let encode = |e: serde_json::Error| {
        host_error(
            ErrorCode::InvalidCanonicalValue,
            "orchestrator.input_invalid",
            e,
        )
    };
    let mut resolved = serde_json::to_value(config).map_err(encode)?;
    resolved
        .as_object_mut()
        .expect("a configuration is an object")
        .remove("canon_id");
    Ok(json!({
        "format": WORLD_MANIFEST_FORMAT,
        "algorithm_version": ALGORITHM_VERSION,
        "protocol": PROTOCOL,
        "config": resolved,
        "constraints": serde_json::to_value(&inputs.constraints).map_err(encode)?,
        "packs": {
            "atlas": sha256_hex(&inputs.atlas_bytes),
            "catalogue": sha256_hex(&inputs.catalogue_bytes),
        },
        "runtime_fingerprint": runtime.fingerprint,
        "runtime": runtime.manifest,
    }))
}
type JobOutcome = Result<Arc<VerifiedPopulation>, HistoryError>;
#[derive(Default)]
struct Job {
    outcome: Mutex<Option<JobOutcome>>,
    finished: Condvar,
}
impl Job {
    fn finish(&self, outcome: JobOutcome) {
        *self.outcome.lock().expect("job lock") = Some(outcome);
        self.finished.notify_all();
    }
    fn wait(&self) -> JobOutcome {
        let mut outcome = self.outcome.lock().expect("job lock");
        loop {
            match outcome.as_ref() {
                Some(done) => return done.clone(),
                None => outcome = self.finished.wait(outcome).expect("job lock"),
            }
        }
    }
}
/// Fails the shared job if its leader unwinds, so that no waiter hangs.
struct JobLeader<'a> {
    jobs: &'a Mutex<BTreeMap<String, Arc<Job>>>,
    domain_id: &'a str,
    job: Arc<Job>,
    finished: bool,
}
impl JobLeader<'_> {
    fn finish(mut self, outcome: JobOutcome) -> JobOutcome {
        // A failed job leaves no trace: the next request replays the same plan (§13).
        if outcome.is_err() {
            self.jobs.lock().expect("jobs lock").remove(self.domain_id);
        }
        self.job.finish(outcome.clone());
        self.finished = true;
        outcome
    }
}
impl Drop for JobLeader<'_> {
    fn drop(&mut self) {
        if !self.finished {
            if let Ok(mut jobs) = self.jobs.lock() {
                jobs.remove(self.domain_id);
            }
            self.job.finish(Err(HistoryError::new(
                ErrorCode::ProviderUnavailable,
                "orchestrator.job_abandoned",
            )));
        }
    }
}
/// Private job root of one run, removed on every exit path.
struct JobRoot(PathBuf);
impl Drop for JobRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
/// One world on one pinned runtime.
pub struct Orchestrator {
    canon: GeneralCanon,
    runtime: RuntimeIdentity,
    store: Store,
    settings: WorkerSettings,
    regions: Mutex<BTreeMap<String, Arc<RegionPlan>>>,
    jobs: Mutex<BTreeMap<String, Arc<Job>>>,
    packets: Mutex<BTreeMap<String, Arc<SocialPacket>>>,
    worker_runs: AtomicUsize,
}
impl Orchestrator {
    /// Pins the runtime, derives `canon_id` from the manifest, builds the canon and
    /// publishes it. Another pinned runtime is another canon, never a silent replay.
    pub fn open(
        inputs: WorldInputs,
        settings: WorkerSettings,
        store: Store,
    ) -> Result<Self, HistoryError> {
        let runtime = query_runtime(&settings)?;
        let mut config = inputs.config.clone();
        let declared = &config.provider.runtime_fingerprint;
        if declared != UNPINNED_RUNTIME && *declared != runtime.fingerprint {
            return Err(host_error(
                ErrorCode::ProviderVersionMismatch,
                "orchestrator.runtime_mismatch",
                &runtime.fingerprint,
            ));
        }
        config.provider.runtime_fingerprint = runtime.fingerprint.clone();
        let mut manifest = world_manifest(&inputs, &config, &runtime)?;
        let fingerprint = sha256_hex(&canonical_bytes(&manifest)?);
        // 128 bits are enough, and this ID is repeated inside every public address.
        config.canon_id = format!("canon-{}", &fingerprint[..32]);
        let canon = build_canon(
            &config,
            &inputs.atlas,
            &inputs.catalogue,
            &inputs.constraints,
        )?;
        let object = manifest.as_object_mut().expect("a manifest is an object");
        object.insert("canon_id".into(), config.canon_id.clone().into());
        object.insert("manifest_fingerprint".into(), fingerprint.into());
        store.publish_world(
            &config.canon_id,
            &canonical_bytes(&manifest)?,
            &canon.canonical_bytes()?,
            &[inputs.atlas_bytes, inputs.catalogue_bytes],
        )?;
        Ok(Self {
            canon,
            runtime,
            store,
            settings,
            regions: Mutex::default(),
            jobs: Mutex::default(),
            packets: Mutex::default(),
            worker_runs: AtomicUsize::new(0),
        })
    }
    pub fn canon(&self) -> &GeneralCanon {
        &self.canon
    }
    pub fn runtime_fingerprint(&self) -> &str {
        &self.runtime.fingerprint
    }
    pub fn store(&self) -> &Store {
        &self.store
    }
    /// Provider processes started by this orchestrator: zero on a warm read.
    pub fn worker_runs(&self) -> usize {
        self.worker_runs.load(Ordering::SeqCst)
    }
    pub fn domain_ids(&self) -> Vec<String> {
        self.canon
            .domains
            .iter()
            .map(|d| d.domain_id.clone())
            .collect()
    }
    /// Regional plan, computed once and published.
    pub fn plan_region(&self, region_id: &str) -> Result<Arc<RegionPlan>, HistoryError> {
        let mut regions = self.regions.lock().expect("regions lock");
        if let Some(region) = regions.get(region_id) {
            return Ok(region.clone());
        }
        let region = Arc::new(plan_region(&self.canon, region_id)?);
        self.store
            .publish_region(&self.canon.canon_id, region_id, &region.canonical_bytes()?)?;
        regions.insert(region_id.to_string(), region.clone());
        Ok(region)
    }
    pub fn region_of(&self, domain_id: &str) -> Result<Arc<RegionPlan>, HistoryError> {
        let domain = self
            .canon
            .domains
            .iter()
            .find(|d| d.domain_id == domain_id)
            .ok_or_else(|| {
                host_error(
                    ErrorCode::InvalidPopulationPlan,
                    "orchestrator.unknown_domain",
                    domain_id,
                )
            })?;
        self.plan_region(&domain.region_id)
    }
    /// Compiled plan with its host-computed fingerprint (§4.2).
    pub fn plan_population(&self, domain_id: &str) -> Result<PopulationPlan, HistoryError> {
        let region = self.region_of(domain_id)?;
        let mut plan = plan_population(&self.canon, &region, domain_id)?;
        plan.plan_fingerprint = sha256_hex(&plan_canonical_bytes(&plan)?);
        Ok(plan)
    }
    fn cache_key(&self, plan: &PopulationPlan) -> Result<PopulationCacheKey, HistoryError> {
        let region = self.region_of(&plan.domain_id)?;
        Ok(PopulationCacheKey {
            canon_id: self.canon.canon_id.clone(),
            owner: plan.domain_id.clone(),
            plan_id: plan.plan_id.clone(),
            plan_fingerprint: plan.plan_fingerprint.clone(),
            region_fingerprint: sha256_hex(&region.canonical_bytes()?),
            core_algorithm: ALGORITHM_VERSION.into(),
            provider_algorithm: plan.provider.algorithm_id.clone(),
            projection_version: plan.provider.projection_version.clone(),
            runtime_fingerprint: plan.provider.runtime_fingerprint.clone(),
            protocol: PROTOCOL.into(),
            result_format: POPULATION_RESULT_FORMAT.into(),
            detail: POPULATION_DETAIL.into(),
        })
    }
    /// Store first, worker otherwise. A warm read goes through the same validation.
    fn resolve(&self, domain_id: &str) -> Result<VerifiedPopulation, HistoryError> {
        let plan = self.plan_population(domain_id)?;
        // §13: every failure names its owner and its plan.
        self.resolve_plan(&plan).map_err(|mut error| {
            error.owner.get_or_insert_with(|| domain_id.to_string());
            error.plan_id.get_or_insert_with(|| plan.plan_id.clone());
            error
        })
    }
    fn resolve_plan(&self, plan: &PopulationPlan) -> Result<VerifiedPopulation, HistoryError> {
        let key = self.cache_key(plan)?;
        if let Some(bytes) = self.store.load_population(&key)? {
            return validate_population(plan, parse_population_result(&bytes)?);
        }
        self.worker_runs.fetch_add(1, Ordering::SeqCst);
        let root = JobRoot(self.store.root().join(".jobs").join(unique_name()));
        let artifact = run_plan(&self.settings, plan, &root.0)?;
        let verified = validate_population(plan, parse_population_result(&artifact.bytes)?)?;
        self.store
            .publish_population(&key, &artifact, verified.proof())?;
        Ok(verified)
    }
    /// The domain's population, whatever the order or concurrency of requests: one
    /// job per domain, whose outcome every concurrent caller receives.
    pub fn population(&self, domain_id: &str) -> JobOutcome {
        let (job, leads) = {
            let mut jobs = self.jobs.lock().expect("jobs lock");
            match jobs.get(domain_id) {
                Some(job) => (job.clone(), false),
                None => {
                    let job = Arc::new(Job::default());
                    jobs.insert(domain_id.to_string(), job.clone());
                    (job, true)
                }
            }
        };
        if !leads {
            return job.wait();
        }
        let leader = JobLeader {
            jobs: &self.jobs,
            domain_id,
            job,
            finished: false,
        };
        leader.finish(self.resolve(domain_id).map(Arc::new))
    }
    /// Full-horizon social packet of a domain, built once from its verified population
    /// (§14.1). The lock is held while building: one packet is computed once.
    pub fn social_packet(&self, domain_id: &str) -> Result<Arc<SocialPacket>, HistoryError> {
        let mut packets = self.packets.lock().expect("packets lock");
        if let Some(packet) = packets.get(domain_id) {
            return Ok(packet.clone());
        }
        let population = self.population(domain_id)?;
        let region = self.region_of(domain_id)?;
        let packet = Arc::new(build_social_packet(&region, &population)?);
        packets.insert(domain_id.to_string(), packet.clone());
        Ok(packet)
    }
}
