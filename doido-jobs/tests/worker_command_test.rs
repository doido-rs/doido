//! Worker CLI smoke test — drains the memory queue once and exits.

use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn worker_run_once_with_memory_backend_exits() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "jobs:\n  type: memory\n  concurrency: 1\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");

    doido_jobs::commands::worker::run(true).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
}
