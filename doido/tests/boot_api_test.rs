//! Public boot API (`install_runtime_globals`, `BootOptions`).

use doido::{install_runtime_globals, BootOptions};
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());
static BOOT_API_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn install_runtime_globals_from_app_config_dir() {
    let _boot = BOOT_API_LOCK.lock().unwrap();
    let _guard = CWD_LOCK.lock().unwrap();
    let _pool = doido_model::pool::test_lock();
    if doido::storage::try_storage().is_some() {
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

    assert!(doido::storage::try_storage().is_some());
    install_runtime_globals(&BootOptions::new()).await;
}
