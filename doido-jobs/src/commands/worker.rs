use doido_jobs::{JobContext, JobRegistry, WorkerEngine};
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
    let mut job_ctx = JobContext::new();
    if let Some(conn) = doido_model::pool::try_pool() {
        job_ctx.insert(conn.clone());
    }
    let engine = WorkerEngine::with_context(queue, config.engine_config(), job_ctx);

    // Every `#[job]` registers its handler at link time; build the lookup once and
    // route each reserved payload to its handler by `job_name`. An unknown name
    // returns `Err`, so the engine retries and eventually dead-letters it rather
    // than silently acking work with no handler.
    let registry = Arc::new(JobRegistry::from_inventory());
    doido_core::tracing::info!("registered jobs: {:?}", registry.names());
    let handler = move |job, ctx| {
        let registry = Arc::clone(&registry);
        async move { registry.dispatch(job, ctx).await }
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
