//! Process-global default deliverer, built from the `mailer` config.
//!
//! Mirrors `doido-cache`'s global store, but builds lazily and synchronously
//! (constructing a deliverer needs no async work): the first call to
//! [`deliverer`] reads `config/<env>.yml` and installs the configured backend,
//! so `Mail::deliver_later`'s background job always has a deliverer even without
//! an explicit boot step. Call [`set_deliverer`] (e.g. in tests) to override.

use crate::deliverer::Deliverer;
use std::sync::{Arc, OnceLock};

static DELIVERER: OnceLock<Arc<dyn Deliverer>> = OnceLock::new();

/// The global deliverer, lazily built from `[mailer]` config on first use.
pub fn deliverer() -> Arc<dyn Deliverer> {
    DELIVERER
        .get_or_init(|| crate::config::load().build())
        .clone()
}

/// Eagerly build and install the deliverer (surfaces config early at boot).
pub fn init() -> Arc<dyn Deliverer> {
    deliverer()
}

/// Install a specific deliverer as the global default (e.g. a `TestDeliverer`).
/// Returns `Err` with the deliverer back if one was already installed.
pub fn set_deliverer(deliverer: Arc<dyn Deliverer>) -> std::result::Result<(), Arc<dyn Deliverer>> {
    DELIVERER.set(deliverer)
}
