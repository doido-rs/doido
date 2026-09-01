//! `doido jobs` CLI commands against the in-memory backend.

use doido_jobs::commands::jobs::{run, JobsCommand};
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn jobs_failed_with_empty_dead_store() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "jobs:\n  type: memory\n  queues: [default]\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");

    run(JobsCommand::Failed).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
}

#[tokio::test]
async fn jobs_retry_and_discard_on_empty_store() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "jobs:\n  type: memory\n  queues: [default]\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");

    run(JobsCommand::Retry).await;
    run(JobsCommand::Discard).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
}
