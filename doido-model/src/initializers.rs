//! Framework default initializer for database pool management.

use crate::config;
use crate::pool::{self, pool_options};
use crate::sea_orm::Database;
use doido_core::Result;

/// Execution contexts detected by `default_initializer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionContext {
    /// Server mode (`doido server` or bare app binary): initializes global pool via `pool::init().await`.
    Server,
    /// Worker mode (`doido worker`): initializes global pool with worker settings via `pool::set_pool(conn)`.
    Worker,
    /// Commands that manage their own database connection or do not need an application pool
    /// (e.g. `db migrate`, `db seed`, `generate`, test runners).
    Skip,
}

/// Detects the execution context based on environment variables and CLI arguments.
pub fn detect_execution_context() -> ExecutionContext {
    // 1. Explicit environment variable override (useful for tests and containerized setups)
    if let Ok(cmd) = std::env::var("DOIDO_COMMAND") {
        match cmd.to_lowercase().as_str() {
            "server" => return ExecutionContext::Server,
            "worker" => return ExecutionContext::Worker,
            _ => return ExecutionContext::Skip,
        }
    }

    // 2. Test runner detection: if running under `cargo test` / test harness, skip by default
    if is_test_runner() {
        return ExecutionContext::Skip;
    }

    // 3. Inspect command-line arguments: find first non-flag argument after executable
    let subcmd = std::env::args().skip(1).find(|arg| !arg.starts_with('-'));

    match subcmd.as_deref() {
        Some("worker") => ExecutionContext::Worker,
        Some("server") | None => ExecutionContext::Server,
        // Any other command (db, generate, routes, console, credentials, config, new, jobs...)
        // does not need the application database pool installed at initializer time
        _ => ExecutionContext::Skip,
    }
}

/// Heuristic to detect if running under `cargo test` / test harness.
fn is_test_runner() -> bool {
    if std::env::var("DOIDO_ENV").as_deref() == Ok("test") {
        return true;
    }
    if std::env::args().any(|a| a == "--test" || a == "--bench") {
        return true;
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy();
        if exe_str.contains("/deps/") || exe_str.contains(r"\deps\") {
            return true;
        }
    }
    false
}

/// Runs a future synchronously and safely whether inside a multi-thread Tokio runtime,
/// a single-thread Tokio runtime, or outside Tokio entirely.
#[doc(hidden)]
pub fn run_async<F>(future: F) -> Result<()>
where
    F: std::future::Future<Output = Result<()>> + Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(future))
            }
            _ => std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| {
                            doido_core::anyhow::anyhow!("failed to create tokio runtime: {e}")
                        })?;
                    rt.block_on(future)
                })
                .join()
                .unwrap_or_else(|_| Err(doido_core::anyhow::anyhow!("runtime thread panicked")))
            }),
        },
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| doido_core::anyhow::anyhow!("failed to create tokio runtime: {e}"))?;
            rt.block_on(future)
        }
    }
}

/// Initializes the database pool for the specified [`ExecutionContext`].
pub fn init_for_context(ctx: ExecutionContext) -> Result<()> {
    match ctx {
        ExecutionContext::Skip => Ok(()),
        ExecutionContext::Server => {
            if pool::try_pool().is_some() {
                return Ok(());
            }
            run_async(async {
                pool::init().await.map_err(|e| {
                    doido_core::anyhow::anyhow!("database pool initialization failed: {e}")
                })?;
                Ok(())
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
