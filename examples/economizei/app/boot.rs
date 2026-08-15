use crate::jobs::import_authorized_banks_job::{
    import_authorized_banks_job_enqueue, ImportAuthorizedBanksPayload,
};
use doido_core::Environment;
use std::sync::Arc;
use std::time::Duration;

/// Schedules startup jobs when the HTTP server boots. Skipped in test and for
/// non-server CLI commands (`db migrate`, `worker`, etc.).
pub fn schedule_startup_jobs_if_server() {
    if is_test_environment() || !is_server_command() {
        return;
    }

    tokio::spawn(async {
        if let Err(error) = run_startup_jobs().await {
            doido_core::tracing::error!(error = %error, "startup jobs failed");
        }
    });
}

fn is_test_environment() -> bool {
    matches!(Environment::get_env(), Environment::Test)
}

fn is_server_command() -> bool {
    matches!(std::env::args().nth(1).as_deref(), None | Some("server"))
}

async fn run_startup_jobs() -> doido_core::Result<()> {
    wait_for_database_pool().await?;

    let config = doido_jobs::config::load();
    let queue = doido_jobs::config::build_configured_queue(&config).await?;

    import_authorized_banks_job_enqueue(
        &*queue,
        ImportAuthorizedBanksPayload {},
    )
    .await?;

    drain_ready_jobs(&config, queue).await
}

async fn wait_for_database_pool() -> doido_core::Result<()> {
    for _ in 0..100 {
        if doido_model::pool::try_pool().is_some() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    Err(doido_core::anyhow::anyhow!(
        "database pool was not ready for startup jobs"
    ))
}

async fn drain_ready_jobs(
    config: &doido_jobs::JobsConfig,
    queue: Arc<dyn doido_jobs::JobQueue>,
) -> doido_core::Result<()> {
    let mut job_ctx = doido_jobs::JobContext::new();
    if let Some(conn) = doido_model::pool::try_pool() {
        job_ctx.insert(conn.clone());
    }

    let engine = doido_jobs::WorkerEngine::with_context(
        queue,
        config.engine_config(),
        job_ctx,
    );
    let registry = Arc::new(doido_jobs::JobRegistry::from_inventory());
    let handler = move |job, ctx| {
        let registry = Arc::clone(&registry);
        async move { registry.dispatch(job, ctx).await }
    };

    while engine.run_once(&handler).await? {}

    Ok(())
}
