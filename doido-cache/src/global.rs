//! Process-global default cache store, built once from config at boot.
//!
//! Mirrors `doido-model`'s connection pool: [`init`] builds the store described
//! by the current environment's `cache` config and installs it; handlers read it
//! via [`store`]. Apps that want several named stores can still use
//! [`crate::CacheRegistry`] directly.

use crate::store::CacheStore;
use doido_core::Result;
use std::sync::{Arc, OnceLock};

static CACHE: OnceLock<Arc<dyn CacheStore>> = OnceLock::new();

/// Builds the cache store from `config/<env>.yml` and installs it as the global
/// default, returning a handle. Idempotent: if already initialised, the existing
/// store is returned and the freshly built one discarded. Call once at boot.
pub async fn init() -> Result<Arc<dyn CacheStore>> {
    if let Some(existing) = CACHE.get() {
        return Ok(existing.clone());
    }
    let store = crate::config::load().build().await?;
    let _ = CACHE.set(store);
    Ok(CACHE.get().expect("cache was just set").clone())
}

/// Installs an already-built store as the global default (e.g. in tests).
/// Returns `Err` with the store back if one was already installed.
pub fn set_store(store: Arc<dyn CacheStore>) -> std::result::Result<(), Arc<dyn CacheStore>> {
    CACHE.set(store)
}

/// Returns the global cache store, panicking if [`init`]/[`set_store`] was never
/// called. Use from request handlers where boot is guaranteed to have run.
pub fn store() -> Arc<dyn CacheStore> {
    CACHE
        .get()
        .expect("cache not initialised; call doido_cache::global::init() at boot")
        .clone()
}

/// Returns the global cache store if installed, else `None`.
pub fn try_store() -> Option<Arc<dyn CacheStore>> {
    CACHE.get().cloned()
}
