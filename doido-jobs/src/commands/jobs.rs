//! `doido jobs` — run handlers, list registrations, and manage the dead-letter store.
//!
//! Backed by the configured queue backend (memory/db/redis), selected from the
//! `jobs` section of `config/<env>.yml` (in-memory when absent) — the same
//! backend the worker drains.

use crate::{JobPayload, JobQueue, JobsConfig};
use clap::Subcommand;

use super::run::{self as run_cmd, RunArgs};

#[derive(Subcommand)]
pub enum JobsCommand {
    /// List registered `#[job]` handler names
    List,
    /// Run a job in-process (`perform_now`) or enqueue with `--later`
    #[command(visible_alias = "perform")]
    Run(RunArgs),
    /// List failed (dead-lettered) jobs
    Failed,
    /// Retry every failed job (re-enqueue, then clear the dead-letter store)
    Retry,
    /// Discard every failed job
    Discard,
}

/// Run a `doido jobs` subcommand. Returns a process exit code (`0` = success).
pub async fn run(cmd: JobsCommand) -> i32 {
    match cmd {
        JobsCommand::List => list_jobs(),
        JobsCommand::Run(args) => {
            // `build_configured_queue` installs the DB pool for the `db` backend (same as worker).
            let _config = crate::config::load();
            let queue = match crate::config::build_configured_queue(&_config).await {
                Ok(q) => Some(q),
                Err(e) => {
                    doido_core::tracing::error!("failed to build jobs backend: {e}");
                    eprintln!("error: failed to build jobs backend: {e:#}");
                    return 1;
                }
            };
            run_cmd::run(args, queue).await
        }
        JobsCommand::Failed => {
            let Some((queue, config)) = configured_queue().await else {
                return 1;
            };
            list_failed(queue.as_ref(), &config).await;
            0
        }
        JobsCommand::Retry => {
            let Some((queue, config)) = configured_queue().await else {
                return 1;
            };
            retry_failed(queue.as_ref(), &config).await;
            0
        }
        JobsCommand::Discard => {
            let Some((queue, config)) = configured_queue().await else {
                return 1;
            };
            discard_failed(queue.as_ref(), &config).await;
            0
        }
    }
}

async fn configured_queue() -> Option<(std::sync::Arc<dyn JobQueue>, JobsConfig)> {
    let config = crate::config::load();
    match crate::config::build_configured_queue(&config).await {
        Ok(q) => Some((q, config)),
        Err(e) => {
            doido_core::tracing::error!("failed to build jobs backend: {e}");
            None
        }
    }
}

fn list_jobs() -> i32 {
    let registry = crate::JobRegistry::from_inventory();
    for name in registry.names() {
        println!("{name}");
    }
    0
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
