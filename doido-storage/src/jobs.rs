//! Background storage jobs (feature `storage-jobs`) — the `PurgeJob` / `AnalyzeJob`
//! analogues, built on `doido-jobs`.
//!
//! Each `#[job]` handler rebuilds a [`Storage`](crate::Storage) from the current
//! environment's config and the process-global DB pool, so a worker can dispatch
//! it by queue name. The generated `*_enqueue` helpers (and the `*_later`
//! wrappers here) push work onto any [`JobQueue`].

use doido_core::Result;
use doido_jobs::{JobId, JobQueue};
use serde::{Deserialize, Serialize};

/// Payload for a deferred blob purge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurgeBlob {
    /// The blob storage key to purge.
    pub key: String,
}

/// Payload for a deferred blob analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeBlob {
    /// The blob storage key to analyze.
    pub key: String,
}

/// Purge a blob in the background. Generates `storage_purge_enqueue`.
#[doido_jobs::job(queue = "storage_purge", max_retries = 5)]
pub async fn storage_purge(payload: PurgeBlob) -> Result<()> {
    let conn = doido_model::pool::pool().clone();
    let storage = crate::Storage::from_config(conn).await?;
    storage.purge(&payload.key).await
}

/// Analyze a blob in the background. Generates `storage_analyze_enqueue`.
///
/// Metadata analysis (image dimensions, etc.) is deferred; this is the hook a
/// future analyzer plugs into.
#[doido_jobs::job(queue = "storage_analyze")]
pub async fn storage_analyze(payload: AnalyzeBlob) -> Result<()> {
    let _ = payload;
    Ok(())
}

/// Enqueue a purge for `key` (Rails `blob.purge_later`).
pub async fn purge_later(queue: &dyn JobQueue, key: &str) -> Result<JobId> {
    storage_purge_enqueue(queue, PurgeBlob { key: key.to_string() }).await
}

/// Enqueue an analysis for `key` (Rails `blob.analyze_later`).
pub async fn analyze_later(queue: &dyn JobQueue, key: &str) -> Result<JobId> {
    storage_analyze_enqueue(queue, AnalyzeBlob { key: key.to_string() }).await
}
