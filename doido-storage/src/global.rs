//! Process-global default storage facade, built once from config at boot.
//!
//! Mirrors the framework boot pattern: [`init`] builds the facade described
//! by the current environment's `storage` config and installs it; handlers read it
//! via [`storage`].

use crate::client::Storage;
use crate::config;
use doido_core::Result;
use std::sync::OnceLock;

static STORAGE: OnceLock<Storage> = OnceLock::new();

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
    let storage = cfg
        .into_storage(
            doido_model::pool::pool().clone(),
            crate::signing::Signer::from_env(),
        )
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
