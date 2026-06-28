use doido_jobs::{build_queue, JobPayload, JobsConfig, WorkerEngine};

/// Start the background worker.
///
/// Builds the configured queue backend (memory/db/redis) behind an
/// `Arc<dyn JobQueue>` and runs the backend-agnostic [`WorkerEngine`] until the
/// process receives Ctrl-C, at which point in-flight jobs are drained.
pub async fn run() {
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
        "starting background worker (backend={:?}, queues={:?}, concurrency={})",
        config.backend,
        config.queues,
        config.concurrency,
    );

    let engine = WorkerEngine::new(queue, config.engine_config());

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        doido_core::tracing::info!("shutdown signal received, draining in-flight jobs...");
    };

    // TODO: dispatch to the registered job handler. A job-type registry (mapping
    // each `#[job]` to its `perform`) is required for real execution; until then
    // the worker logs each reserved job and acks it.
    let handler = |job: JobPayload| async move {
        doido_core::tracing::info!("processing job {} on queue {}", job.id, job.queue);
        Ok(())
    };

    if let Err(e) = engine.run(handler, shutdown).await {
        doido_core::tracing::error!("worker engine error: {e}");
    }
}
