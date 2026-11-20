//! Process-global default deliverer, built from the `mailer` config.
//!
//! Mirrors `doido-cache`'s global store, but builds lazily and synchronously
//! (constructing a deliverer needs no async work): the first call to
//! [`deliverer`] reads `config/<env>.yml` and installs the configured backend,
//! so `Mail::deliver_later`'s background job always has a deliverer even without
//! an explicit boot step. Call [`set_deliverer`] (e.g. in tests) to override.

use crate::deliverer::Deliverer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

static DELIVERER: OnceLock<Arc<dyn Deliverer>> = OnceLock::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Resets the default initializer state for testing.
#[doc(hidden)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::SeqCst);
}

/// Framework default initializer registered for `mailer` (priority 35).
///
/// Discovers execution context and eagerly installs the configured [`Deliverer`].
pub fn default_initializer() -> doido_core::Result<()> {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let ctx = doido_core::initializers::detect_execution_context();
    if ctx.is_skip() {
        return Ok(());
    }

    let _ = deliverer();
    Ok(())
}

inventory::submit! {
    doido_core::initializers::FrameworkInitializer {
        name: "mailer",
        priority: 35,
        init: default_initializer,
    }
}

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
