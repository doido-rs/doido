//! Batch enqueue / schedule / in-process perform (Rails `perform_later` / `perform_now`).

use crate::context::JobContext;
use crate::queue::{JobId, JobPayload, JobQueue};
use crate::registry::JobRegistry;
use chrono::{DateTime, Utc};
use doido_core::Result;
use std::sync::Arc;

/// Enqueue multiple jobs for the worker (`perform_later`). Each payload keeps its
/// own `run_at` (use the fluent builder's `wait` / `at` before building payloads).
pub async fn enqueue_jobs(
    queue: &dyn JobQueue,
    jobs: impl IntoIterator<Item = JobPayload>,
) -> Result<Vec<JobId>> {
    let jobs: Vec<_> = jobs.into_iter().collect();
    if jobs.is_empty() {
        return Ok(Vec::new());
    }
    queue.enqueue_many(jobs).await
}

/// Schedule multiple jobs, all eligible no earlier than `at`.
pub async fn enqueue_jobs_at(
    queue: &dyn JobQueue,
    jobs: impl IntoIterator<Item = JobPayload>,
    at: DateTime<Utc>,
) -> Result<Vec<JobId>> {
    let jobs: Vec<_> = jobs.into_iter().collect();
    if jobs.is_empty() {
        return Ok(Vec::new());
    }
    queue.enqueue_many_at(jobs, at).await
}

/// Run multiple handlers in this process (`perform_now`), sequentially. Stops on
/// the first error or timeout.
pub async fn perform_jobs_now(
    registry: &JobRegistry,
    jobs: impl IntoIterator<Item = JobPayload>,
    ctx: Arc<JobContext>,
) -> Result<()> {
    for job in jobs {
        crate::worker::perform_job_now(registry, job, Arc::clone(&ctx)).await?;
    }
    Ok(())
}
