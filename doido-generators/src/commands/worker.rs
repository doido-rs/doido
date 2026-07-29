use doido_jobs::{JobContext, JobPayload, WorkerEngine};
use std::sync::Arc;

/// Start the background worker.
///
/// Builds the configured queue backend (memory/db/redis) behind an
/// `Arc<dyn JobQueue>` and runs the backend-agnostic [`WorkerEngine`]. With
/// `once`, it drains the jobs currently ready and exits (cron-friendly);
/// otherwise it runs until the process receives Ctrl-C, draining in-flight jobs.
pub async fn run(once: bool) {
    // Backend + queues + concurrency come from the `jobs` section of
    // `config/<env>.yml` (in-memory when absent). The `db` backend connects
    // using the app's `database` config.
    let config = doido_jobs::config::load();

    let queue = match doido_jobs::config::build_configured_queue(&config).await {
        Ok(q) => q,
        Err(e) => {
            doido_core::tracing::error!("failed to build jobs backend: {e}");
            return;
        }
    };

    doido_core::tracing::info!(
        "starting background worker (backend={:?}, queues={:?}, concurrency={}, once={once})",
        config.backend,
        config.queues,
        config.concurrency,
    );

    // The engine carries the application context handed to every job handler.
    let engine = WorkerEngine::with_context(queue, config.engine_config(), JobContext::new());

    // TODO: dispatch to the registered job handler. A job-type registry (mapping
    // each `#[job]` to its `perform(payload, ctx)`) is required for real execution;
    // until then the worker logs each reserved job and acks it. `ctx` is the
    // shared application context the engine carries.
    let handler = |job: JobPayload, _ctx: Arc<JobContext>| async move {
        doido_core::tracing::info!("processing job {} on queue {}", job.id, job.queue);
        Ok(())
    };

    if once {
        // Drain everything ready right now, then exit.
        loop {
            match engine.run_once(&handler).await {
                Ok(true) => continue,
                Ok(false) => break,
                Err(e) => {
                    doido_core::tracing::error!("worker engine error: {e}");
                    break;
                }
            }
        }
        doido_core::tracing::info!("worker drained ready jobs, exiting (once)");
        return;
    }

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        doido_core::tracing::info!("shutdown signal received, draining in-flight jobs...");
    };
    if let Err(e) = engine.run(handler, shutdown).await {
        doido_core::tracing::error!("worker engine error: {e}");
    }
}
