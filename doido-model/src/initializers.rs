//! Framework default initializer for database pool management.

use crate::config;
use crate::pool::{self, pool_options};
use crate::sea_orm::Database;
use doido_core::Result;

pub use doido_core::initializers::{
    detect_execution_context, is_test_runner, run_async, ExecutionContext,
};

/// Initializes the database pool for the specified [`ExecutionContext`].
pub fn init_for_context(ctx: ExecutionContext) -> Result<()> {
    match ctx {
        ExecutionContext::Skip => Ok(()),
        ExecutionContext::Server | ExecutionContext::Console => {
            if pool::try_pool().is_some() {
                return Ok(());
            }
            let env = doido_core::Environment::get_env();
            run_async(async move {
                match pool::init().await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        if env.is_production() {
                            Err(doido_core::anyhow::anyhow!(
                                "database pool initialization failed in production: {e}"
                            ))
                        } else {
                            doido_core::tracing::warn!("database pool initialization failed: {e}");
                            Ok(())
                        }
                    }
                }
            })
        }
        ExecutionContext::Worker => {
            if pool::try_pool().is_some() {
                return Ok(());
            }
            run_async(async {
                let config = config::load();
                let db = config.database();
                let opts = pool_options(
                    db.url.as_str(),
                    config.logger().sql,
                    db.pool,
                    db.connect_timeout,
                );
                let conn = Database::connect(opts).await.map_err(|e| {
                    doido_core::anyhow::anyhow!("worker database connection failed: {e}")
                })?;
                let _ = pool::set_pool(conn);
                Ok(())
            })
        }
    }
}

/// Framework default initializer for `doido-model`.
///
/// Automatically detects whether the application is running in `server`, `worker`,
/// or a skipped command context (`db migrate`, `generate`, tests, etc.), executes
/// the connection asynchronously within the current runtime, and ensures idempotency.
pub fn default_initializer() -> Result<()> {
    init_for_context(detect_execution_context())
}

inventory::submit! {
    doido_core::initializers::FrameworkInitializer {
        name: "db",
        priority: 10,
        init: default_initializer,
    }
}
