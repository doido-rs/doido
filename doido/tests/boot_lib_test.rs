//! Boot module integration tests (kept out of `src/boot.rs` so llvm-cov does not
//! count test lines against the crate gate).

use doido::{install_runtime_globals, install_test_runtime_globals, BootOptions};
use doido_core::boot::install_i18n;
use doido_core::i18n::{register_entry, reset_for_test, test_guard, translate, DEFAULT_LOCALE};
use doido_core::{BootPolicy, InitPolicy};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

static BOOT_TEST_LOCK: Mutex<()> = Mutex::new(());

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
async fn storage_skipped_when_pool_cannot_be_installed() {
    let _boot = BOOT_TEST_LOCK.lock().unwrap();
    let _lock = doido_model::pool::test_lock();
    if doido_storage::try_storage().is_some() {
        return;
    }
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "jobs:\n  type: memory\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");
    std::env::remove_var("DATABASE_URL");

    install_runtime_globals(&BootOptions::new()).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
    assert!(doido_storage::try_storage().is_none());
}

#[tokio::test]
async fn storage_installs_from_database_config_yaml() {
    let _boot = BOOT_TEST_LOCK.lock().unwrap();
    let _lock = doido_model::pool::test_lock();
    if doido_storage::try_storage().is_some() {
        return;
    }
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "database:\n  url: \"sqlite::memory:\"\nstorage:\n  driver: local\n  drivers:\n    local: { type: disk, root: tmp/storage }\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");

    install_runtime_globals(&BootOptions::new()).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
    assert!(doido_storage::try_storage().is_some());
}

#[test]
fn boot_options_default_new_and_builder() {
    let default = BootOptions::default();
    assert!(default.storage_config_loader.is_none());
    assert_eq!(BootOptions::new().policy, default.policy);

    let options = BootOptions::new()
        .boot_policy(BootPolicy {
            i18n: InitPolicy::FailFast,
            storage: InitPolicy::Warn,
        })
        .storage_config_loader(Box::new(doido_storage::config::load));
    assert_eq!(options.policy.i18n, InitPolicy::FailFast);
    assert_eq!(options.policy.storage, InitPolicy::Warn);
    assert!(options.storage_config_loader.is_some());
}

#[tokio::test]
async fn install_runtime_globals_short_circuits_when_storage_ready() {
    let _boot = BOOT_TEST_LOCK.lock().unwrap();
    let _lock = doido_model::pool::test_lock();
    if doido_storage::try_storage().is_none() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("config")).unwrap();
        fs::write(
            dir.path().join("config/test.yml"),
            "database:\n  url: \"sqlite::memory:\"\nstorage:\n  driver: local\n  drivers:\n    local: { type: disk, root: tmp/storage }\n",
        )
        .unwrap();
        let original_dir = std::env::current_dir().unwrap();
        let original_env = std::env::var("DOIDO_ENV").ok();
        std::env::set_current_dir(dir.path()).unwrap();
        std::env::set_var("DOIDO_ENV", "test");
        install_runtime_globals(&BootOptions::new()).await;
        std::env::set_current_dir(original_dir).unwrap();
        if let Some(v) = original_env {
            std::env::set_var("DOIDO_ENV", v);
        } else {
            std::env::remove_var("DOIDO_ENV");
        }
    }
    assert!(doido_storage::try_storage().is_some());
    install_runtime_globals(&BootOptions::new()).await;
}

#[tokio::test]
async fn storage_install_is_idempotent_with_loader() {
    let _boot = BOOT_TEST_LOCK.lock().unwrap();
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

#[tokio::test]
async fn install_test_runtime_globals_installs_pool_and_storage() {
    let _boot = BOOT_TEST_LOCK.lock().unwrap();
    let _lock = doido_model::pool::test_lock();
    if doido_storage::try_storage().is_some() {
        return;
    }
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "database:\n  url: \"sqlite::memory:\"\nstorage:\n  driver: local\n  drivers:\n    local: { type: disk, root: tmp/storage }\n",
    )
    .unwrap();
    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");

    let conn = doido_model::connect_with_url("sqlite::memory:")
        .await
        .unwrap();
    install_test_runtime_globals(conn).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
    assert!(doido_model::pool::try_pool().is_some());
    assert!(doido_storage::try_storage().is_some());
}
