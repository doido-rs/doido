use doido_jobs::{build_queue, JobPayload, JobsConfig, WorkerEngine};

/// Start the background worker.
///
/// Builds the configured queue backend (memory/db/redis) behind an
/// `Arc<dyn JobQueue>` and runs the backend-agnostic [`WorkerEngine`]. With
/// `once`, it drains the jobs currently ready and exits (cron-friendly);
/// otherwise it runs until the process receives Ctrl-C, draining in-flight jobs.
pub async fn run(once: bool) {
    // TODO: load `[jobs]` from the application config once the config crate
    // exposes it here. Until then the engine runs against the in-memory backend.
    let config = JobsConfig::default();

    let queue = match build_queue(&config).await {
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

    let engine = WorkerEngine::new(queue, config.engine_config());

    // TODO: dispatch to the registered job handler. A job-type registry (mapping
    // each `#[job]` to its `perform`) is required for real execution; until then
    // the worker logs each reserved job and acks it.
    let handler = |job: JobPayload| async move {
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
