//! Runtime globals shared by `server`, `worker`, and other CLI entrypoints.

use doido_core::boot::{handle_init_error, install_i18n, BootPolicy, DEFAULT_LOCALES_DIR};
use doido_model::sea_orm::DatabaseConnection;
use doido_storage::config::{resolve_for_boot, StorageConfigLoader};
use std::path::Path;
use std::sync::Arc;

/// Boot-time options registered on the [`crate::Doido`] builder.
#[derive(Clone, Default)]
pub struct BootOptions {
    pub policy: BootPolicy,
    pub storage_config_loader: Option<Arc<StorageConfigLoader>>,
}

impl BootOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn boot_policy(mut self, policy: BootPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn storage_config_loader(mut self, loader: StorageConfigLoader) -> Self {
        self.storage_config_loader = Some(Arc::new(loader));
        self
    }
}

/// Install i18n and the global storage facade (idempotent). Ensures the model
/// pool exists when storage needs a database connection.
pub async fn install_runtime_globals(options: &BootOptions) {
    install_i18n(Path::new(DEFAULT_LOCALES_DIR), options.policy.i18n);

    if doido_storage::try_storage().is_some() {
        return;
    }

    if doido_model::pool::try_pool().is_none() {
        match doido_model::pool::init().await {
            Ok(_) => {}
            Err(e) => {
                handle_init_error(
                    options.policy.storage,
                    "failed to connect to the database for storage",
                    e,
                );
                if doido_model::pool::try_pool().is_none() {
                    return;
                }
            }
        }
    }

    let cfg = resolve_for_boot(options.storage_config_loader.as_deref());
    if let Err(e) = doido_storage::init_from_config(cfg).await {
        handle_init_error(options.policy.storage, "failed to initialize storage", e);
    }
}

/// Test harness: install a connection, then i18n + storage (same as server/worker boot).
pub async fn install_test_runtime_globals(conn: DatabaseConnection) {
    install_test_runtime_globals_with(conn, &BootOptions::new()).await;
}

/// Like [`install_test_runtime_globals`] with custom [`BootOptions`].
pub async fn install_test_runtime_globals_with(conn: DatabaseConnection, options: &BootOptions) {
    if doido_model::pool::try_pool().is_none() {
        let _ = doido_model::pool::set_pool(conn);
    }
    install_runtime_globals(options).await;
}
