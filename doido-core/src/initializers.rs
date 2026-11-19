//! Framework default initializers for core runtime.

use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Default framework initializer for `doido-core`.
///
/// Idempotently sets up basic logging/tracing and loads the `.env` file when
/// running in the [`crate::Environment::Development`] environment.
pub fn default_initializer() -> crate::Result<()> {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    // 1. Basic logging / tracing if not already configured
    crate::logger::init();

    // 2. Load .env in development
    if crate::Environment::get_env() == crate::Environment::Development {
        let _ = dotenvy::dotenv();
    }

    Ok(())
}

#[doc(hidden)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::SeqCst);
}
