use crate::cron;
use crate::{JobRegistry, WorkerEngine};
use std::sync::Arc;

/// Start the background worker.
///
/// Builds the configured queue backend (memory/db/redis) behind an
/// `Arc<dyn JobQueue>` and runs the backend-agnostic [`WorkerEngine`]. With
/// `once`, it drains the jobs currently ready and exits (cron-friendly);
/// otherwise it runs until the process receives Ctrl-C, draining in-flight jobs.
///
/// For the `db` backend, [`build_configured_queue`](crate::config::build_configured_queue)
/// installs the global model pool when absent (same as the HTTP server). Successful
/// jobs are removed from `doido_jobs` (`ack`); failures update the row in place
/// (`nack` / `dead_letter`) with the error preserved for inspection.
pub async fn run(once: bool) {
    // Backend + queues + concurrency come from the `jobs` section of
    // `config/<env>.yml` (in-memory when absent). The `db` backend installs
    // the global database pool via `doido_model::pool::init()` when needed.
    let config = crate::config::load();

    let queue = match crate::config::build_configured_queue(&config).await {
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

    let job_ctx = crate::context::job_context_from_runtime();
    let queue_refs: Vec<&str> = config.queues.iter().map(|s| s.as_str()).collect();

    // Every `#[job]` registers its handler at link time; build the lookup once and
    // route each reserved payload to its handler by `job_name`. An unknown name
    // returns `Err`, so the engine retries and eventually dead-letters it rather
    // than silently acking work with no handler.
    let registry = Arc::new(JobRegistry::from_inventory());
    doido_core::tracing::info!("registered jobs: {:?}", registry.names());

    if once {
        let handler = {
            let registry = Arc::clone(&registry);
            move |job, ctx| {
                let registry = Arc::clone(&registry);
                async move { registry.dispatch(job, ctx).await }
            }
        };
        match queue.reclaim_expired(&queue_refs).await {
            Ok(n) if n > 0 => doido_core::tracing::info!("reclaimed {n} expired job(s)"),
            Ok(_) => {}
            Err(e) => doido_core::tracing::error!("reclaim_expired before drain: {e}"),
        }
        let engine = WorkerEngine::with_context(queue, config.engine_config(), job_ctx);
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

    let handler = {
        let registry = Arc::clone(&registry);
        move |job, ctx| {
            let registry = Arc::clone(&registry);
            async move { registry.dispatch(job, ctx).await }
        }
    };

    let engine = WorkerEngine::with_context(Arc::clone(&queue), config.engine_config(), job_ctx);

    let cron_handles = match cron::boot(
        &config.cron,
        &config,
        Arc::clone(&queue),
        Arc::clone(&registry),
    )
    .await
    {
        Ok(handles) => {
            if !handles.is_empty() {
                doido_core::tracing::info!("cron scheduler started ({} task(s))", handles.len());
            }
            handles
        }
        Err(e) => {
            doido_core::tracing::error!("failed to load jobs.cron: {e:#}");
            return;
        }
    };

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        doido_core::tracing::info!("shutdown signal received, draining in-flight jobs...");
    };
    if let Err(e) = engine.run(handler, shutdown).await {
        doido_core::tracing::error!("worker engine error: {e}");
    }

    for handle in cron_handles {
        handle.abort();
    }
}
