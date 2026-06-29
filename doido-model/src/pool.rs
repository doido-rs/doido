//! Database connections backed by the model [`config`](crate::config).
//!
//! Besides the one-shot [`connect`]/[`connect_with_url`] helpers, this module
//! holds a process-global connection installed once at application boot
//! ([`init`]) and read from request handlers via [`pool`]. Controllers reach it
//! through `Context::db()`.

use crate::config;
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::sync::OnceLock;

/// Process-global connection, installed by [`init`]/[`set_pool`].
static POOL: OnceLock<DatabaseConnection> = OnceLock::new();

/// Builds [`ConnectOptions`] with sea-orm's SQL statement logging enabled, so
/// queries surface through the global tracing subscriber (target `sqlx::query`,
/// level `INFO`). Tune visibility with `RUST_LOG` (see `doido_core::logger`).
fn options(url: &str) -> ConnectOptions {
    let mut opts = ConnectOptions::new(url.to_owned());
    // Enabled by default in sea-orm, but set explicitly to centralize intent.
    // Statements log at the default `INFO` level under target `sqlx::query`.
    opts.sqlx_logging(true);
    opts
}

/// Connects to the database named by the current environment's
/// `config/<env>.yml` `database.url`.
pub async fn connect() -> Result<DatabaseConnection, DbErr> {
    let config = config::load();
    Database::connect(options(config.database().url.as_str())).await
}

/// Connects using an explicit database URL, bypassing the config file.
pub async fn connect_with_url(url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(options(url)).await
}

/// Connects (via [`connect`]) and installs the process-global pool, returning a
/// reference to it. Idempotent: if already initialised, the existing connection
/// is returned and the new one discarded. Call once at server boot.
pub async fn init() -> Result<&'static DatabaseConnection, DbErr> {
    if let Some(existing) = POOL.get() {
        return Ok(existing);
    }
    let conn = connect().await?;
    let _ = POOL.set(conn);
    Ok(POOL.get().expect("pool was just set"))
}

/// Installs an already-open connection as the global pool (e.g. in tests).
/// Returns `Err` with the connection back if one was already installed.
pub fn set_pool(conn: DatabaseConnection) -> Result<(), DatabaseConnection> {
    POOL.set(conn)
}

/// Returns the global connection, panicking if [`init`]/[`set_pool`] was never
/// called. Use from request handlers where boot is guaranteed to have run.
pub fn pool() -> &'static DatabaseConnection {
    POOL.get().expect(
        "database pool not initialised; call doido_model::pool::init() at boot before handling requests",
    )
}

/// Returns the global connection if installed, else `None`.
pub fn try_pool() -> Option<&'static DatabaseConnection> {
    POOL.get()
}

/// A process-global lock for serializing tests that share the global pool.
///
/// The pool is process-global, so tests that install it and run requests must
/// not execute concurrently. Generated scaffold controller tests hold this lock
/// for their duration; hold it in any hand-written test that touches the global
/// pool. Poisoning is ignored so one failing test doesn't cascade.
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_pool_then_pool_returns_it() {
        // No pool before install.
        assert!(try_pool().is_none());

        let conn = connect_with_url("sqlite::memory:").await.unwrap();
        set_pool(conn).expect("first install succeeds");

        // Now both accessors see it; a second install is rejected.
        assert!(try_pool().is_some());
        let _ = pool();
        let second = connect_with_url("sqlite::memory:").await.unwrap();
        assert!(set_pool(second).is_err());
    }
}
