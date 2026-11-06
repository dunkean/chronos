//! Lineage Lab child process: JSONL over stdin/stdout, argument list only, logs on
//! stderr (§13). A failure is typed and leaves no partial result; there is no
//! provider fallback.
use super::{host_error, sha256_hex};
use chronos_core::v2::canonical::canonical_bytes;
use chronos_core::v2::model::{ErrorCode, HistoryError};
use chronos_core::v2::population::{
    plan_canonical_bytes, result_canonical_bytes, PopulationPlan, POPULATION_RESULT_FORMAT,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

pub const PROTOCOL: &str = "chronos-lineage-1";
pub const PYTHON_ENV: &str = "CHRONOS_PYTHON";
pub const LINEAGE_SRC_ENV: &str = "CHRONOS_LINEAGE_SRC";
/// Far above the measured 185 s of the largest domain. A timeout is a failure and
/// never selects another scenario (§13).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1800);
const BRIDGE_MODULE: &str = "lineage_lab.chronos_bridge";

#[derive(Clone, Debug)]
pub struct WorkerSettings {
    pub python: PathBuf,
    pub lineage_src: PathBuf,
    /// JIT cache: performance only, it never enters a fingerprint.
    pub cache_dir: PathBuf,
    pub timeout: Duration,
}
impl WorkerSettings {
    /// Explicit value, then environment, then the default next to `root`.
    pub fn resolve(
        python: Option<PathBuf>,
        lineage_src: Option<PathBuf>,
        root: &Path,
        cache_dir: PathBuf,
    ) -> Self {
        let venv = if cfg!(windows) {
            ["Scripts", "python.exe"]
        } else {
            ["bin", "python"]
        };
        let from_env = |name| std::env::var_os(name).map(PathBuf::from);
        Self {
            python: python
                .or_else(|| from_env(PYTHON_ENV))
                .unwrap_or_else(|| root.join(".venv").join(venv[0]).join(venv[1])),
            lineage_src: lineage_src
                .or_else(|| from_env(LINEAGE_SRC_ENV))
                .unwrap_or_else(|| root.join("..").join("lineage-lab").join("src")),
            cache_dir,
            timeout: DEFAULT_TIMEOUT,
        }
    }
    fn command(&self) -> Result<Command, HistoryError> {
        let unavailable = |key: &str, path: &Path| {
            host_error(ErrorCode::ProviderUnavailable, key, path.display())
        };
        if !self.python.is_file() {
            return Err(unavailable("worker.interpreter_missing", &self.python));
        }
        if !self.lineage_src.join("lineage_lab").is_dir() {
            return Err(unavailable(
                "worker.provider_source_missing",
                &self.lineage_src,
            ));
        }
        let absolute = |path: &Path| {
            std::path::absolute(path)
                .map_err(|e| host_error(ErrorCode::ProviderUnavailable, "worker.spawn_failed", e))
        };
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| host_error(ErrorCode::ProviderUnavailable, "worker.spawn_failed", e))?;
        let cache = absolute(&self.cache_dir)?;
        let mut command = Command::new(absolute(&self.python)?);
        command
            .arg("-m")
            .arg(BRIDGE_MODULE)
            .env("PYTHONPATH", absolute(&self.lineage_src)?)
            .env("NUMBA_CACHE_DIR", &cache)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env_remove("PYTHONHOME")
            // Keeps the project directory out of the provider's module path.
            .current_dir(&cache)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        Ok(command)
    }
}
fn spawned(command: &mut Command) -> Result<Child, HistoryError> {
    command
        .spawn()
        .map_err(|e| host_error(ErrorCode::ProviderUnavailable, "worker.spawn_failed", e))
}
/// Reads stdout on its own thread, so that every wait can honour the timeout.
fn line_reader(stdout: ChildStdout) -> Receiver<std::io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = Vec::new();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) => return,
                Ok(_) => {
                    if sender.send(Ok(line)).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = sender.send(Err(e));
                    return;
                }
            }
        }
    });
    receiver
}
/// Waits for one line. On timeout or a dead worker the child is stopped.
fn next_line(
    child: &mut Child,
    lines: &Receiver<std::io::Result<Vec<u8>>>,
    timeout: Duration,
) -> Result<Vec<u8>, HistoryError> {
    let failure = match lines.recv_timeout(timeout) {
        Ok(Ok(line)) => return Ok(line),
        Ok(Err(e)) => host_error(ErrorCode::ProviderUnavailable, "worker.exited", e),
        Err(RecvTimeoutError::Disconnected) => {
            let status = child.wait().map(|s| s.to_string()).unwrap_or_default();
            host_error(ErrorCode::ProviderUnavailable, "worker.exited", status)
        }
        Err(RecvTimeoutError::Timeout) => host_error(
            ErrorCode::ProviderUnavailable,
            "worker.timeout",
            timeout.as_secs(),
        ),
    };
    let _ = child.kill();
    let _ = child.wait();
    Err(failure)
}

/// Pinned runtime of the installed provider (§4.2).
#[derive(Clone, Debug)]
pub struct RuntimeIdentity {
    pub fingerprint: String,
    pub manifest: Value,
}
pub fn query_runtime(settings: &WorkerSettings) -> Result<RuntimeIdentity, HistoryError> {
    let mut child = spawned(settings.command()?.arg("--runtime").stdin(Stdio::null()))?;
    let lines = line_reader(child.stdout.take().expect("piped stdout"));
    let line = next_line(&mut child, &lines, settings.timeout)?;
    let _ = child.wait();
    let protocol = |e: &dyn ToString| {
        host_error(
            ErrorCode::ProviderUnavailable,
            "worker.protocol_invalid",
            e.to_string(),
        )
    };
    let value: Value = serde_json::from_slice(&line).map_err(|e| protocol(&e))?;
    let fingerprint = value["runtime_fingerprint"]
        .as_str()
        .ok_or_else(|| protocol(&"runtime_fingerprint"))?
        .to_string();
    let manifest = value["runtime"].clone();
    if sha256_hex(&canonical_bytes(&manifest)?) != fingerprint {
        return Err(protocol(&"runtime_fingerprint"));
    }
    Ok(RuntimeIdentity {
        fingerprint,
        manifest,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ResultRef {
    pub relative_path: String,
    pub file_sha256: String,
    pub bytes: u64,
    pub format: String,
}
#[derive(Debug, Deserialize)]
struct ProviderFailure {
    code: String,
    message: Option<String>,
    owner: Option<String>,
    year: Option<i32>,
    constraint: Option<String>,
    plan_id: Option<String>,
}
impl From<ProviderFailure> for HistoryError {
    fn from(failure: ProviderFailure) -> Self {
        // The closed list of §13; anything else is a provider breakdown.
        let code = serde_json::from_value(Value::String(failure.code.clone()))
            .unwrap_or(ErrorCode::ProviderUnavailable);
        HistoryError {
            owner: failure.owner,
            at_year: failure.year,
            plan_id: failure.plan_id,
            ..HistoryError::new(code, "worker.provider_error")
        }
        .with_param("provider_code", failure.code)
        .with_param(
            "provider_constraint",
            failure.constraint.unwrap_or_default(),
        )
        .with_param("detail", failure.message.unwrap_or_default())
    }
}
#[derive(Debug, Deserialize)]
struct Envelope {
    protocol: String,
    request_id: Option<String>,
    ok: bool,
    result_fingerprint: Option<String>,
    error: Option<ProviderFailure>,
    result: Option<PreparedPlan>,
    job_directory: Option<String>,
    result_ref: Option<ResultRef>,
    metrics: Option<Value>,
}
#[derive(Debug, Deserialize)]
struct PreparedPlan {
    payload_ref: String,
    runtime_fingerprint: String,
    result_coverage: String,
    result_format: String,
}
/// Checked provider file: its bytes passed every check of `read_verified_result`.
#[derive(Clone, Debug)]
pub struct ProviderArtifact {
    pub bytes: Vec<u8>,
    pub file_sha256: String,
    pub result_fingerprint: String,
    /// Provider's own duration, reported and never hashed.
    pub materialize_seconds: Option<f64>,
}
fn invalid(key: &str, detail: impl ToString) -> HistoryError {
    host_error(ErrorCode::InvalidPopulationResult, key, detail)
}
/// A worker path is plain relative segments: no root, drive, `..` or stream name.
fn relative_segments(text: &str) -> Result<&Path, HistoryError> {
    let path = Path::new(text);
    let plain = !text.is_empty()
        && !text.contains([':', '\\', '\0'])
        && path.components().all(|c| matches!(c, Component::Normal(_)));
    if plain {
        Ok(path)
    } else {
        Err(invalid("worker.result_path_escapes", text))
    }
}
/// Resolves `relative` under `base`, following links, and refuses to leave it.
fn resolve_inside(base: &Path, relative: &str) -> Result<PathBuf, HistoryError> {
    let joined = base.join(relative_segments(relative)?);
    let resolved =
        std::fs::canonicalize(&joined).map_err(|e| invalid("worker.result_unreadable", e))?;
    if resolved == base || !resolved.starts_with(base) {
        return Err(invalid("worker.result_path_escapes", relative));
    }
    Ok(resolved)
}
/// File canonical as a whole, whose content hashes to `announced` (§4.2).
pub fn check_result_bytes(bytes: &[u8], announced: &str) -> Result<(), HistoryError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| invalid("worker.result_not_canonical", e))?;
    if canonical_bytes(&value).map_err(|e| invalid("worker.result_not_canonical", e))? != bytes {
        return Err(invalid("worker.result_not_canonical", "bytes"));
    }
    if value["result_fingerprint"].as_str() != Some(announced) {
        return Err(invalid("worker.result_fingerprint_differs", announced));
    }
    if sha256_hex(&result_canonical_bytes(value)?) != announced {
        return Err(invalid("worker.result_fingerprint_differs", announced));
    }
    Ok(())
}
/// §13, in order: path inside the job directory, size, SHA-256, then the fingerprint.
pub fn read_verified_result(
    job_root: &Path,
    job_directory: &str,
    reference: &ResultRef,
    announced_fingerprint: &str,
    max_bytes: u64,
) -> Result<ProviderArtifact, HistoryError> {
    let root =
        std::fs::canonicalize(job_root).map_err(|e| invalid("worker.result_unreadable", e))?;
    let directory = resolve_inside(&root, job_directory)?;
    let file = resolve_inside(&directory, &reference.relative_path)?;
    if reference.format != POPULATION_RESULT_FORMAT {
        return Err(invalid("worker.result_format_invalid", &reference.format));
    }
    let on_disk = std::fs::metadata(&file)
        .map_err(|e| invalid("worker.result_unreadable", e))?
        .len();
    if on_disk != reference.bytes {
        return Err(invalid("worker.result_size_differs", on_disk));
    }
    if on_disk > max_bytes {
        return Err(host_error(
            ErrorCode::BudgetExceeded,
            "worker.result_too_large",
            on_disk,
        ));
    }
    let bytes = std::fs::read(&file).map_err(|e| invalid("worker.result_unreadable", e))?;
    let file_sha256 = sha256_hex(&bytes);
    if bytes.len() as u64 != reference.bytes {
        return Err(invalid("worker.result_size_differs", bytes.len()));
    }
    if file_sha256 != reference.file_sha256 {
        return Err(invalid("worker.result_sha256_differs", file_sha256));
    }
    check_result_bytes(&bytes, announced_fingerprint)?;
    Ok(ProviderArtifact {
        bytes,
        file_sha256,
        result_fingerprint: announced_fingerprint.to_string(),
        materialize_seconds: None,
    })
}
/// The plan as sent: canonical list order, so the provider hashes the same bytes.
fn wire_plan(plan: &PopulationPlan) -> Result<Value, HistoryError> {
    let mut value: Value = serde_json::from_slice(&plan_canonical_bytes(plan)?).map_err(|e| {
        host_error(
            ErrorCode::InvalidCanonicalValue,
            "worker.plan_unencodable",
            e,
        )
    })?;
    let object = value
        .as_object_mut()
        .expect("a plan serializes to an object");
    object.insert("plan_id".into(), plan.plan_id.clone().into());
    object.insert(
        "plan_fingerprint".into(),
        plan.plan_fingerprint.clone().into(),
    );
    Ok(value)
}
/// One worker process. It runs one job at a time and dies with this value.
pub struct LineageWorker {
    child: Child,
    requests: Option<Sender<Vec<u8>>>,
    lines: Receiver<std::io::Result<Vec<u8>>>,
    job_root: PathBuf,
    timeout: Duration,
    next_request: u64,
}
impl LineageWorker {
    pub fn spawn(settings: &WorkerSettings, job_root: &Path) -> Result<Self, HistoryError> {
        std::fs::create_dir_all(job_root)
            .map_err(|e| host_error(ErrorCode::ProviderUnavailable, "worker.spawn_failed", e))?;
        let job_root = std::path::absolute(job_root)
            .map_err(|e| host_error(ErrorCode::ProviderUnavailable, "worker.spawn_failed", e))?;
        let mut child = spawned(
            settings
                .command()?
                .arg("--job-root")
                .arg(&job_root)
                .stdin(Stdio::piped()),
        )?;
        let lines = line_reader(child.stdout.take().expect("piped stdout"));
        // A plan can exceed the pipe buffer: writing must not escape the timeout.
        let mut stdin = child.stdin.take().expect("piped stdin");
        let (requests, pending) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            for line in pending {
                if stdin.write_all(&line).and_then(|()| stdin.flush()).is_err() {
                    return;
                }
            }
        });
        Ok(Self {
            child,
            requests: Some(requests),
            lines,
            job_root,
            timeout: settings.timeout,
            next_request: 0,
        })
    }
    fn call(&mut self, method: &str, mut fields: Value) -> Result<Envelope, HistoryError> {
        self.next_request += 1;
        // Request IDs never reach a random draw (§13).
        let request_id = format!("req-{}", self.next_request);
        let object = fields.as_object_mut().expect("request fields");
        object.insert("protocol".into(), PROTOCOL.into());
        object.insert("request_id".into(), request_id.clone().into());
        object.insert("method".into(), method.into());
        let mut line = serde_json::to_vec(&fields).map_err(|e| {
            host_error(
                ErrorCode::InvalidCanonicalValue,
                "worker.plan_unencodable",
                e,
            )
        })?;
        line.push(b'\n');
        let sent = self.requests.as_ref().map(|r| r.send(line));
        if !matches!(sent, Some(Ok(()))) {
            return Err(host_error(
                ErrorCode::ProviderUnavailable,
                "worker.exited",
                method,
            ));
        }
        let raw = next_line(&mut self.child, &self.lines, self.timeout)?;
        let envelope: Envelope = serde_json::from_slice(&raw).map_err(|e| {
            host_error(ErrorCode::ProviderUnavailable, "worker.protocol_invalid", e)
        })?;
        if envelope.protocol != PROTOCOL {
            return Err(host_error(
                ErrorCode::ProviderVersionMismatch,
                "worker.protocol_invalid",
                &envelope.protocol,
            ));
        }
        if !envelope.ok {
            return Err(envelope.error.map(HistoryError::from).unwrap_or_else(|| {
                host_error(
                    ErrorCode::ProviderUnavailable,
                    "worker.protocol_invalid",
                    "error",
                )
            }));
        }
        if envelope.request_id.as_deref() != Some(&request_id) {
            return Err(host_error(
                ErrorCode::ProviderUnavailable,
                "worker.protocol_invalid",
                "request_id",
            ));
        }
        Ok(envelope)
    }
    /// Sends the complete plan (§13) and checks what the worker says it will produce.
    pub fn prepare(&mut self, plan: &PopulationPlan) -> Result<(), HistoryError> {
        let fields = json!({"plan_id": plan.plan_id, "plan": wire_plan(plan)?});
        let prepared = self.call("prepare", fields)?.result.ok_or_else(|| {
            host_error(
                ErrorCode::ProviderUnavailable,
                "worker.protocol_invalid",
                "result",
            )
        })?;
        if prepared.runtime_fingerprint != plan.provider.runtime_fingerprint {
            return Err(host_error(
                ErrorCode::ProviderVersionMismatch,
                "worker.runtime_mismatch",
                prepared.runtime_fingerprint,
            ));
        }
        if prepared.payload_ref != plan.plan_fingerprint
            || prepared.result_coverage != "complete"
            || prepared.result_format != POPULATION_RESULT_FORMAT
        {
            return Err(host_error(
                ErrorCode::ProviderUnavailable,
                "worker.protocol_invalid",
                "prepare",
            ));
        }
        Ok(())
    }
    /// Runs the prepared plan. The job directory is removed once its file is read.
    pub fn materialize(&mut self, plan: &PopulationPlan) -> Result<ProviderArtifact, HistoryError> {
        let fields = json!({"plan_id": plan.plan_id, "payload_ref": plan.plan_fingerprint});
        let envelope = self.call("materialize", fields)?;
        let missing = |field: &str| {
            host_error(
                ErrorCode::ProviderUnavailable,
                "worker.protocol_invalid",
                field,
            )
        };
        let directory = envelope
            .job_directory
            .ok_or_else(|| missing("job_directory"))?;
        let reference = envelope.result_ref.ok_or_else(|| missing("result_ref"))?;
        let fingerprint = envelope
            .result_fingerprint
            .ok_or_else(|| missing("result_fingerprint"))?;
        let verified = read_verified_result(
            &self.job_root,
            &directory,
            &reference,
            &fingerprint,
            plan.limits.max_result_bytes,
        );
        if relative_segments(&directory).is_ok() {
            let _ = std::fs::remove_dir_all(self.job_root.join(&directory));
        }
        let seconds = envelope
            .metrics
            .and_then(|m| m["materialize_seconds"].as_f64());
        verified.map(|artifact| ProviderArtifact {
            materialize_seconds: seconds,
            ..artifact
        })
    }
}
impl Drop for LineageWorker {
    fn drop(&mut self) {
        self.requests = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
/// A plan in a fresh process: the reference isolation of §13.
pub fn run_plan(
    settings: &WorkerSettings,
    plan: &PopulationPlan,
    job_root: &Path,
) -> Result<ProviderArtifact, HistoryError> {
    let mut worker = LineageWorker::spawn(settings, job_root)?;
    worker.prepare(plan)?;
    worker.materialize(plan)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_ref_cannot_leave_the_job_directory() {
        let root = std::env::temp_dir().join(format!("chronos-ref-{}", std::process::id()));
        std::fs::create_dir_all(root.join("job-1")).unwrap();
        std::fs::write(root.join("outside.json"), b"{}").unwrap();
        let reference = |path: &str| ResultRef {
            relative_path: path.into(),
            file_sha256: String::new(),
            bytes: 2,
            format: POPULATION_RESULT_FORMAT.into(),
        };
        for (directory, path) in [
            ("job-1", "../outside.json"),
            ("job-1", "..\\outside.json"),
            ("job-1", "C:/outside.json"),
            ("job-1", "/outside.json"),
            ("job-1", "."),
            ("..", "outside.json"),
            ("job-1/..", "outside.json"),
        ] {
            let error =
                read_verified_result(&root, directory, &reference(path), "x", 10).unwrap_err();
            assert_eq!(error.constraint, "worker.result_path_escapes", "{path}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
