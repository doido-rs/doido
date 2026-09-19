//! Runtime globals shared by `server`, `worker`, and other CLI entrypoints.

use doido_core::boot::{handle_init_error, install_i18n, BootPolicy, DEFAULT_LOCALES_DIR};
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

#[cfg(test)]
mod tests {
    use super::*;
    use doido_core::i18n::{register_entry, reset_for_test, test_guard, translate, DEFAULT_LOCALE};
    use doido_core::InitPolicy;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn i18n_install_is_idempotent() {
        let _g = test_guard();
        reset_for_test();
        register_entry(DEFAULT_LOCALE, "hello", "Hello").unwrap();
        install_i18n(Path::new("/nonexistent/locales"), InitPolicy::Warn);
        install_i18n(Path::new("/nonexistent/locales"), InitPolicy::Warn);
        assert_eq!(translate("hello", DEFAULT_LOCALE), "Hello");
    }

    #[tokio::test]
    async fn storage_install_is_idempotent_with_loader() {
        if doido_storage::try_storage().is_some() {
            return;
        }
        let _lock = doido_model::pool::test_lock();
        if doido_model::pool::try_pool().is_none() {
            let conn = doido_model::connect_with_url("sqlite::memory:")
                .await
                .unwrap();
            let _ = doido_model::pool::set_pool(conn);
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let calls2 = calls.clone();
        let options = BootOptions::new().storage_config_loader(Box::new(move || {
            calls2.fetch_add(1, Ordering::SeqCst);
            doido_storage::config::load()
        }));
        install_runtime_globals(&options).await;
        install_runtime_globals(&options).await;
        assert!(doido_storage::try_storage().is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
