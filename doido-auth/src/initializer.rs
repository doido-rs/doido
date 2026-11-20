//! Framework default initializer for authentication subsystem, templates, and locales.

use doido_core::Result;
use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Resets the initializer state for testing.
#[doc(hidden)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::SeqCst);
    crate::controllers::state::reset_state();
}

/// Framework default initializer registered for `auth` (priority 25).
///
/// Registers built-in i18n locales, framework view templates, mounts application views,
/// inspects execution context, and initializes auth state with the database pool.
pub fn default_initializer() -> Result<()> {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    // 1. Register built-in auth locales
    if let Err(e) = crate::i18n::register_locales() {
        doido_core::tracing::warn!("auth locales registration: {e}");
    }

    // 2. Register framework views and initialize application views
    crate::framework_views::register_views();

    let view_dir = if std::path::Path::new("views").exists() {
        "views"
    } else if std::path::Path::new("app/views").exists() {
        "app/views"
    } else if std::path::Path::new("backend/app/views").exists() {
        "backend/app/views"
    } else {
        "app/views"
    };

    if let Err(e) = doido_view::init(view_dir) {
        doido_core::tracing::warn!("failed to load views from {view_dir}: {e}");
    }

    // 3. CLI awareness: skip in migration and administrative contexts
    let ctx = doido_core::cli_context();
    if ctx.is_skip() {
        return Ok(());
    }

    // 4. Database pool coupling & AuthState initialization
    doido_core::initializers::run_async(async {
        let Some(db) = doido_model::pool::try_pool().cloned() else {
            doido_core::tracing::warn!("auth default_initializer: database pool is not available");
            return Ok(());
        };
        let config = crate::config::load();
        if let Err(e) = crate::controllers::state::init(db, &config).await {
            doido_core::tracing::warn!("failed to initialise auth: {e}");
        }
        Ok(())
    })
}

inventory::submit! {
    doido_core::initializers::FrameworkInitializer {
        name: "auth",
        priority: 25,
        init: default_initializer,
    }
}
