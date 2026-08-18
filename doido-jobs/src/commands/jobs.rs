//! `doido jobs` — inspect and manage the dead-letter store.
//!
//! Backed by the configured queue backend (memory/db/redis), selected from the
//! `jobs` section of `config/<env>.yml` (in-memory when absent) — the same
//! backend the worker drains.

use clap::Subcommand;
use doido_jobs::{JobPayload, JobQueue, JobsConfig};

#[derive(Subcommand)]
pub enum JobsCommand {
    /// List failed (dead-lettered) jobs
    Failed,
    /// Retry every failed job (re-enqueue, then clear the dead-letter store)
    Retry,
    /// Discard every failed job
    Discard,
}

pub async fn run(cmd: JobsCommand) {
    let config = doido_jobs::config::load();
    let queue = match doido_jobs::config::build_configured_queue(&config).await {
        Ok(q) => q,
        Err(e) => {
            doido_core::tracing::error!("failed to build jobs backend: {e}");
            return;
        }
    };
    match cmd {
        JobsCommand::Failed => list_failed(queue.as_ref(), &config).await,
        JobsCommand::Retry => retry_failed(queue.as_ref(), &config).await,
        JobsCommand::Discard => discard_failed(queue.as_ref(), &config).await,
    }
}

/// Print every dead-lettered job across the configured queues.
async fn list_failed(queue: &dyn JobQueue, config: &JobsConfig) {
    let mut total = 0;
    for q in &config.queues {
        match queue.dead_jobs(q).await {
            Ok(jobs) => {
                for job in &jobs {
                    total += 1;
                    doido_core::tracing::info!(
                        "[{}] {} attempts={} error={}",
                        job.queue,
                        job.id,
                        job.attempts,
                        job.error.as_deref().unwrap_or("-"),
                    );
                }
            }
            Err(e) => doido_core::tracing::error!("failed to read dead jobs for {q}: {e}"),
        }
    }
    if total == 0 {
        doido_core::tracing::info!("failed jobs: (none)");
    } else {
        doido_core::tracing::info!("{total} failed job(s)");
    }
}

/// Re-enqueue every dead job (Rails-style: a fresh run with a new id, so it does
/// not collide with the dead row still in the store), then clear the dead store.
async fn retry_failed(queue: &dyn JobQueue, config: &JobsConfig) {
    let mut retried = 0;
    for q in &config.queues {
        let jobs = match queue.dead_jobs(q).await {
            Ok(j) => j,
            Err(e) => {
                doido_core::tracing::error!("failed to read dead jobs for {q}: {e}");
                continue;
            }
        };
        for job in jobs {
            // A fresh run: `JobPayload::new` mints a new id (status Pending,
            // attempts 0) so it doesn't collide with the dead row still in the
            // store, and we carry the original policy over via the setters.
            let fresh = JobPayload::new(job.queue.clone(), job.payload.clone(), job.max_retries)
                .with_priority(job.priority)
                .with_backoff(job.backoff, job.backoff_base)
                .with_timeout(job.timeout);
            match queue.enqueue(fresh).await {
                Ok(_) => retried += 1,
                Err(e) => doido_core::tracing::error!("re-enqueue failed: {e}"),
            }
        }
        if let Err(e) = queue.discard_dead(q).await {
            doido_core::tracing::error!("failed to clear dead store for {q}: {e}");
        }
    }
    doido_core::tracing::info!("retried {retried} failed job(s)");
}

/// Discard every dead job across the configured queues.
async fn discard_failed(queue: &dyn JobQueue, config: &JobsConfig) {
    let mut discarded = 0;
    for q in &config.queues {
        match queue.discard_dead(q).await {
            Ok(n) => discarded += n,
            Err(e) => doido_core::tracing::error!("failed to discard dead jobs for {q}: {e}"),
        }
    }
    doido_core::tracing::info!("discarded {discarded} failed job(s)");
}
