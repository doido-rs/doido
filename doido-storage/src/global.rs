//! Process-global default storage facade, built once from config at boot.
//!
//! Mirrors the framework boot pattern: [`init`] builds the facade described
//! by the current environment's `storage` config and installs it; handlers read it
//! via [`storage`].

use crate::client::Storage;
use crate::config;
use doido_core::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static STORAGE: OnceLock<Storage> = OnceLock::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Resets the default initializer state for testing.
#[doc(hidden)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::SeqCst);
}

/// Framework default initializer registered for `storage` (priority 20).
///
/// Configures the active storage facade based on the environment configuration,
/// performing safe CLI context detection, async runtime bridging, and environment-aware
/// fault tolerance (fail-fast in production, warnings in development/test).
pub fn default_initializer() -> Result<()> {
    if try_storage().is_some() {
        return Ok(());
    }

    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let ctx = doido_core::initializers::detect_execution_context();
    if ctx.is_skip() {
        return Ok(());
    }

    let env = doido_core::Environment::get_env();

    let init_result = doido_core::initializers::run_async(async {
        if doido_model::pool::try_pool().is_none() {
            let _ = doido_model::pool::init().await;
        }
        let cfg = config::load();
        init_from_config(cfg).await?;
        Ok(())
    });

    match init_result {
        Ok(()) => Ok(()),
        Err(e) => {
            if env.is_production() {
                Err(doido_core::anyhow::anyhow!(
                    "storage initialization failed in production: {e}"
                ))
            } else {
                tracing::warn!("storage default_initializer: {e}");
                Ok(())
            }
        }
    }
}

inventory::submit! {
    doido_core::initializers::FrameworkInitializer {
        name: "storage",
        priority: 20,
        init: default_initializer,
    }
}

/// Builds the storage facade from `config/<env>.yml` and installs it as the global
/// default, returning a handle. Idempotent: if already initialised, the existing
/// facade is returned and the freshly built one discarded. Call once at boot after
/// the database pool is ready.
pub async fn init() -> Result<Storage> {
    init_from_config(config::load()).await
}

/// Builds and installs a storage facade from an explicit config.
pub async fn init_from_config(cfg: config::StorageConfig) -> Result<Storage> {
    if let Some(existing) = STORAGE.get() {
        return Ok(existing.clone());
    }
    let conn = match doido_model::pool::try_pool() {
        Some(p) => p.clone(),
        None => {
            return Err(doido_core::anyhow::anyhow!(
                "database pool not initialised; cannot build storage"
            ));
        }
    };
    let storage = cfg
        .into_storage(conn, crate::signing::Signer::from_env())
        .await?;
    let _ = STORAGE.set(storage);
    Ok(STORAGE.get().expect("storage was just set").clone())
}

/// Installs an already-built facade as the global default (e.g. in tests).
/// Idempotent: returns the existing facade if one is already installed.
pub async fn init_with(storage: Storage) -> Result<Storage> {
    if let Some(existing) = STORAGE.get() {
        return Ok(existing.clone());
    }
    let _ = STORAGE.set(storage.clone());
    Ok(storage)
}

/// Installs an already-built facade as the global default (e.g. in tests).
/// Returns `Err` with the facade back if one was already installed.
pub fn set_storage(storage: Storage) -> std::result::Result<(), Storage> {
    STORAGE.set(storage)
}

/// Returns the global storage facade, panicking if [`init`]/[`set_storage`] was never
/// called. Use from request handlers where boot is guaranteed to have run.
pub fn storage() -> Storage {
    STORAGE
        .get()
        .expect("storage not initialised; call doido_storage::init_storage() at boot")
        .clone()
}

/// Returns the global storage facade if installed, else `None`.
pub fn try_storage() -> Option<Storage> {
    STORAGE.get().cloned()
}

/// Returns the active [`Service`](crate::Service) from the global facade.
pub fn service() -> std::sync::Arc<dyn crate::Service> {
    storage().service().clone()
}
