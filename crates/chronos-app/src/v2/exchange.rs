//! Export and import of a world (§16.2). `canon_only` is independent of the zones
//! visited; `with_materialized` adds the provider archives already computed, sorted by
//! domain, with the fingerprint of the packet built from each.
//!
//! An imported archive is checked structurally and by fingerprint, then published to the
//! store. The provider is not rerun: the report says so, and the archive is never shown
//! as a determinism proof.
use super::orchestrator::{Orchestrator, WorldInputs};
use super::{host_error, sha256_hex};
use chronos_core::v2::canonical::canonical_bytes;
use chronos_core::v2::general::{AuthorConstraints, WorldConfig};
use chronos_core::v2::model::{ErrorCode, HistoryError};
use serde_json::{json, Value};

pub const EXPORT_FORMAT: &str = "chronos-world-export-1";

fn bad(key: &str, detail: impl ToString) -> HistoryError {
    host_error(ErrorCode::InvalidCanonicalValue, key, detail)
}
fn text(bytes: &[u8], what: &str) -> Result<String, HistoryError> {
    String::from_utf8(bytes.to_vec()).map_err(|e| bad("exchange.not_utf8", format!("{what}: {e}")))
}

/// Deterministic bytes of the export. Two exports of the same world with the same
/// materialized domains are identical.
pub fn export_world(
    orchestrator: &Orchestrator,
    inputs: &WorldInputs,
    with_materialized: bool,
) -> Result<Vec<u8>, HistoryError> {
    let canon = orchestrator.canon();
    let canon_bytes = canon.canonical_bytes()?;
    let mut materialized = Vec::new();
    if with_materialized {
        let mut domains = orchestrator.ready_domains();
        domains.sort();
        for domain in domains {
            let held = orchestrator.held(&domain)?;
            let archive = orchestrator
                .store()
                .load_population(&orchestrator.cache_key_of(&domain)?)?
                .ok_or_else(|| bad("exchange.archive_missing", &domain))?;
            let packet_bytes = held.packet.canonical_bytes()?;
            materialized.push(json!({
                "domain_id": domain,
                "plan_id": held.packet.plan_id,
                "plan_fingerprint": held.packet.plan_fingerprint,
                "result_fingerprint": held.packet.result_fingerprint,
                "archive_sha256": sha256_hex(&archive),
                "archive_bytes": archive.len(),
                "packet_sha256": sha256_hex(&packet_bytes),
                "packet_bytes": packet_bytes.len(),
                "archive_text": text(&archive, "archive")?,
            }));
        }
    }
    let bundle = json!({
        "format": EXPORT_FORMAT,
        "mode": if with_materialized { "with_materialized" } else { "canon_only" },
        "canon_id": canon.canon_id,
        "runtime_fingerprint": orchestrator.runtime_fingerprint(),
        "config": serde_json::to_value(&inputs.config).map_err(|e| bad("exchange.encode", e))?,
        "constraints": serde_json::to_value(&inputs.constraints).map_err(|e| bad("exchange.encode", e))?,
        "atlas_text": text(&inputs.atlas_bytes, "atlas")?,
        "catalogue_text": text(&inputs.catalogue_bytes, "catalogue")?,
        "atlas_sha256": sha256_hex(&inputs.atlas_bytes),
        "catalogue_sha256": sha256_hex(&inputs.catalogue_bytes),
        "canon_sha256": sha256_hex(&canon_bytes),
        "canon_bytes": canon_bytes.len(),
        "materialized": materialized,
    });
    canonical_bytes(&bundle)
}

/// Result of an import, kept in host metadata and never inside the facts.
pub struct ImportReport {
    pub canon_id: String,
    pub canon_replayed: bool,
    pub domains: Vec<Value>,
}
impl ImportReport {
    pub fn to_json(&self) -> Value {
        json!({
            "canon_id": self.canon_id,
            "canon_replayed": self.canon_replayed,
            "domains": self.domains,
        })
    }
}

fn field<'a>(bundle: &'a Value, name: &str) -> Result<&'a Value, HistoryError> {
    bundle
        .get(name)
        .ok_or_else(|| bad("exchange.field_missing", name))
}
fn string<'a>(bundle: &'a Value, name: &str) -> Result<&'a str, HistoryError> {
    field(bundle, name)?
        .as_str()
        .ok_or_else(|| bad("exchange.field_missing", name))
}

/// Rebuilds the inputs of an export and checks their fingerprints. Nothing is trusted
/// before it is recomputed: the pack fingerprints, then (in `import_world`) the canon.
pub fn read_inputs(bundle: &Value) -> Result<WorldInputs, HistoryError> {
    if string(bundle, "format")? != EXPORT_FORMAT {
        return Err(bad("exchange.format_unknown", string(bundle, "format")?));
    }
    let atlas = string(bundle, "atlas_text")?.as_bytes().to_vec();
    let catalogue = string(bundle, "catalogue_text")?.as_bytes().to_vec();
    if sha256_hex(&atlas) != string(bundle, "atlas_sha256")?
        || sha256_hex(&catalogue) != string(bundle, "catalogue_sha256")?
    {
        return Err(bad("exchange.pack_fingerprint_differs", "packs"));
    }
    let config: WorldConfig = serde_json::from_value(field(bundle, "config")?.clone())
        .map_err(|e| bad("exchange.field_invalid", format!("config: {e}")))?;
    let constraints: AuthorConstraints =
        serde_json::from_value(field(bundle, "constraints")?.clone())
            .map_err(|e| bad("exchange.field_invalid", format!("constraints: {e}")))?;
    WorldInputs::from_parts(config, constraints, atlas, catalogue)
}

/// Replays the general canon, then adopts each archive after its structural and
/// fingerprint checks. The packet built from an archive must match the exported
/// fingerprint, which proves the same facts without rerunning the provider.
pub fn import_world(
    bundle_bytes: &[u8],
    open: impl FnOnce(WorldInputs) -> Result<Orchestrator, HistoryError>,
) -> Result<(Orchestrator, WorldInputs, ImportReport), HistoryError> {
    let bundle: Value = serde_json::from_slice(bundle_bytes)
        .map_err(|e| bad("exchange.field_invalid", format!("json: {e}")))?;
    let inputs = read_inputs(&bundle)?;
    let orchestrator = open(inputs.clone())?;
    // The canon ID covers the pinned runtime: another runtime gives another canon, never a
    // silent replay of the exported one.
    if orchestrator.canon().canon_id != string(&bundle, "canon_id")? {
        return Err(host_error(
            ErrorCode::ProviderVersionMismatch,
            "exchange.canon_differs",
            format!(
                "exported {}, rebuilt {}",
                string(&bundle, "canon_id")?,
                orchestrator.canon().canon_id
            ),
        ));
    }
    if sha256_hex(&orchestrator.canon().canonical_bytes()?) != string(&bundle, "canon_sha256")? {
        return Err(bad("exchange.canon_differs", "canonical bytes"));
    }
    let mut domains = Vec::new();
    let empty = Vec::new();
    for entry in field(&bundle, "materialized")?.as_array().unwrap_or(&empty) {
        let domain = string(entry, "domain_id")?;
        let archive = string(entry, "archive_text")?.as_bytes();
        if sha256_hex(archive) != string(entry, "archive_sha256")? {
            return Err(bad("exchange.archive_fingerprint_differs", domain));
        }
        orchestrator.adopt_population(domain, archive, string(entry, "result_fingerprint")?)?;
        let packet = orchestrator.social_packet(domain)?;
        let packet_sha = sha256_hex(&packet.canonical_bytes()?);
        let same = packet_sha == string(entry, "packet_sha256")?;
        if !same {
            return Err(bad("exchange.packet_differs", domain));
        }
        domains.push(json!({
            "domain_id": domain,
            "archive_verified": true,
            "packet_matches": same,
            "provider_replayed": false,
        }));
    }
    let report = ImportReport {
        canon_id: orchestrator.canon().canon_id.clone(),
        canon_replayed: true,
        domains,
    };
    Ok((orchestrator, inputs, report))
}
