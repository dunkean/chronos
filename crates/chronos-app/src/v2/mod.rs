//! Social v2 host: the only place that runs the provider, hashes and touches files
//! (§2). `chronos-core` stays pure and receives finished envelopes.
pub mod exchange;
pub mod lineage_worker;
pub mod orchestrator;
pub mod sample;
pub mod server;
pub mod store;

use chronos_core::v2::model::{ErrorCode, HistoryError};
use sha2::{Digest, Sha256};

/// SHA-256 in lower-case hex, the form of every fingerprint (§4.2).
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
/// Host failure with a stable diagnostic key and an optional technical detail.
fn host_error(code: ErrorCode, constraint: &str, detail: impl ToString) -> HistoryError {
    HistoryError::new(code, constraint).with_param("detail", detail.to_string())
}
