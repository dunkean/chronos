//! Immutable JSON packages under `worlds/<canon>/` (§16.1). A package becomes
//! visible by one rename, after it is complete, and is never rewritten.
use super::lineage_worker::ProviderArtifact;
use super::{host_error, sha256_hex};
use chronos_core::v2::canonical::canonical_bytes;
use chronos_core::v2::model::{ErrorCode, HistoryError};
use chronos_core::v2::population::PopulationProof;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const POPULATION_PACKAGE_FORMAT: &str = "chronos-population-package-1";
const PACKAGE_MANIFEST: &str = "manifest.json";
const PROVIDER_ARCHIVE: &str = "provider-archive";

/// Staging names only need to differ inside this process tree.
static STAGING: AtomicU64 = AtomicU64::new(0);
pub fn unique_name() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        STAGING.fetch_add(1, Ordering::Relaxed)
    )
}
fn store_error(key: &str, detail: impl ToString) -> HistoryError {
    host_error(ErrorCode::ProviderUnavailable, key, detail)
}
/// File name of an opaque ID (§16.1): its verified fingerprint. Safe whatever the ID
/// holds (separators, reserved device names, case-only differences, any length). The
/// real ID lives in the package manifest and is compared on every read.
pub fn file_key(id: &str) -> String {
    sha256_hex(id.as_bytes())
}
/// Everything a population package depends on (§16.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationCacheKey {
    pub canon_id: String,
    /// Owner of the package: the domain.
    pub owner: String,
    pub plan_id: String,
    pub plan_fingerprint: String,
    pub region_fingerprint: String,
    pub core_algorithm: String,
    pub provider_algorithm: String,
    pub projection_version: String,
    pub runtime_fingerprint: String,
    pub protocol: String,
    pub result_format: String,
    pub detail: String,
}
impl PopulationCacheKey {
    pub fn fingerprint(&self) -> Result<String, HistoryError> {
        let value = serde_json::to_value(self).map_err(|e| store_error("store.write_failed", e))?;
        Ok(sha256_hex(&canonical_bytes(&value)?))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PopulationPackage {
    format: String,
    cache_key: String,
    key: PopulationCacheKey,
    result_fingerprint: String,
    file_sha256: String,
    bytes: u64,
    validator: String,
}
fn write_durable(path: &Path, bytes: &[u8]) -> Result<(), HistoryError> {
    let mut file = std::fs::File::create(path).map_err(|e| store_error("store.write_failed", e))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| store_error("store.write_failed", e))
}
#[derive(Clone, Debug)]
pub struct Store {
    root: PathBuf,
}
impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn world_dir(&self, canon_id: &str) -> PathBuf {
        self.root.join("worlds").join(file_key(canon_id))
    }
    fn staging_dir(&self, canon_id: &str) -> Result<PathBuf, HistoryError> {
        let parent = self.world_dir(canon_id).join(".staging");
        std::fs::create_dir_all(&parent).map_err(|e| store_error("store.write_failed", e))?;
        Ok(parent.join(unique_name()))
    }
    /// Publishes one immutable file. Republishing the same bytes changes nothing;
    /// other bytes under the same name are a conflict, never an overwrite.
    fn publish_file(&self, canon_id: &str, path: &Path, bytes: &[u8]) -> Result<(), HistoryError> {
        let conflict_free = |path: &Path| match std::fs::read(path) {
            Ok(existing) if existing == bytes => Ok(true),
            Ok(_) => Err(host_error(
                ErrorCode::InvalidRecordIdentity,
                "store.immutable_conflict",
                path.display(),
            )),
            Err(_) => Ok(false),
        };
        if conflict_free(path)? {
            return Ok(());
        }
        let parent = path.parent().expect("package files have a parent");
        std::fs::create_dir_all(parent).map_err(|e| store_error("store.write_failed", e))?;
        let staging = self.staging_dir(canon_id)?;
        write_durable(&staging, bytes)?;
        let renamed = std::fs::rename(&staging, path);
        if let Err(error) = renamed {
            let _ = std::fs::remove_file(&staging);
            // A concurrent publisher may have won with the same immutable bytes.
            if !conflict_free(path)? {
                return Err(store_error("store.write_failed", error));
            }
        }
        Ok(())
    }
    /// World manifest, canon and the frozen packs they cite.
    pub fn publish_world(
        &self,
        canon_id: &str,
        manifest: &[u8],
        canon: &[u8],
        packs: &[Vec<u8>],
    ) -> Result<(), HistoryError> {
        let world = self.world_dir(canon_id);
        for pack in packs {
            let path = world
                .join("packs")
                .join(format!("{}.json", sha256_hex(pack)));
            self.publish_file(canon_id, &path, pack)?;
        }
        self.publish_file(canon_id, &world.join("canon.json"), canon)?;
        self.publish_file(canon_id, &world.join("manifest.json"), manifest)
    }
    pub fn publish_region(
        &self,
        canon_id: &str,
        region_id: &str,
        plan: &[u8],
    ) -> Result<(), HistoryError> {
        let path = self
            .world_dir(canon_id)
            .join("regions")
            .join(format!("{}.json", file_key(region_id)));
        self.publish_file(canon_id, &path, plan)
    }
    fn population_dir(&self, key: &PopulationCacheKey) -> PathBuf {
        self.world_dir(&key.canon_id)
            .join("populations")
            .join(file_key(&key.plan_id))
    }
    /// Whether a package is published, without reading it.
    pub fn has_population(&self, key: &PopulationCacheKey) -> bool {
        self.population_dir(key).join(PACKAGE_MANIFEST).is_file()
    }
    /// Provider file of a published package, rechecked from its manifest. `None`
    /// when nothing is published. A damaged package is an error: population tables
    /// are authoritative data and are never silently replaced (§16.1).
    pub fn load_population(
        &self,
        key: &PopulationCacheKey,
    ) -> Result<Option<Vec<u8>>, HistoryError> {
        let directory = self.population_dir(key);
        let manifest = match std::fs::read(directory.join(PACKAGE_MANIFEST)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(store_error("store.read_failed", e)),
        };
        let corrupt = |detail: &str| {
            host_error(
                ErrorCode::InvalidPopulationResult,
                "store.package_corrupt",
                detail,
            )
        };
        let package: PopulationPackage =
            serde_json::from_slice(&manifest).map_err(|_| corrupt("manifest"))?;
        if package.format != POPULATION_PACKAGE_FORMAT {
            return Err(corrupt("format"));
        }
        if package.key != *key || package.cache_key != key.fingerprint()? {
            return Err(host_error(
                ErrorCode::InvalidRecordIdentity,
                "store.cache_key_mismatch",
                &key.plan_id,
            ));
        }
        let bytes = std::fs::read(directory.join(PROVIDER_ARCHIVE))
            .map_err(|e| store_error("store.read_failed", e))?;
        // The same bytes as the ones checked before publication.
        if bytes.len() as u64 != package.bytes || sha256_hex(&bytes) != package.file_sha256 {
            return Err(corrupt("provider-archive"));
        }
        Ok(Some(bytes))
    }
    /// Publishes a validated population. The proof is the entry ticket: only a
    /// result that passed `validate_population` reaches this function.
    pub fn publish_population(
        &self,
        key: &PopulationCacheKey,
        artifact: &ProviderArtifact,
        proof: &PopulationProof,
    ) -> Result<(), HistoryError> {
        let target = self.population_dir(key);
        let parent = target.parent().expect("population parent");
        std::fs::create_dir_all(parent).map_err(|e| store_error("store.write_failed", e))?;
        let package = PopulationPackage {
            format: POPULATION_PACKAGE_FORMAT.into(),
            cache_key: key.fingerprint()?,
            key: key.clone(),
            result_fingerprint: artifact.result_fingerprint.clone(),
            file_sha256: artifact.file_sha256.clone(),
            bytes: artifact.bytes.len() as u64,
            validator: proof.validator.clone(),
        };
        let manifest = serde_json::to_value(&package)
            .map_err(|e| store_error("store.write_failed", e))
            .and_then(|value| canonical_bytes(&value))?;
        let staging = self.staging_dir(&key.canon_id)?;
        std::fs::create_dir_all(&staging).map_err(|e| store_error("store.write_failed", e))?;
        write_durable(&staging.join(PROVIDER_ARCHIVE), &artifact.bytes)?;
        write_durable(&staging.join(PACKAGE_MANIFEST), &manifest)?;
        // The whole package appears at once, or not at all.
        if let Err(error) = std::fs::rename(&staging, &target) {
            let _ = std::fs::remove_dir_all(&staging);
            let existing = std::fs::read(target.join(PACKAGE_MANIFEST));
            if existing.ok().as_deref() != Some(manifest.as_slice()) {
                return Err(store_error("store.write_failed", error));
            }
        }
        Ok(())
    }
}
