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

#[tokio::test]
async fn jobs_exits_when_backend_cannot_be_built() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "jobs:\n  type: db\n  queues: [default]\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");
    std::env::remove_var("DATABASE_URL");

    run(JobsCommand::Failed).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
}

#[cfg(feature = "jobs-db")]
#[tokio::test]
async fn jobs_retry_requeues_dead_lettered_job_in_db_backend() {
    use doido_jobs::JobPayload;
    use serde_json::json;

    let _guard = CWD_LOCK.lock().unwrap();
    let _pool = doido_model::pool::test_lock();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "database:\n  url: \"sqlite::memory:\"\njobs:\n  type: db\n  queues: [default]\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");

    let cfg = doido_jobs::config::load();
    let queue = doido_jobs::config::build_configured_queue(&cfg)
        .await
        .unwrap();
    let job = JobPayload::new("default", json!({"k": 1}), 1);
    queue.enqueue(job).await.unwrap();
    let reserved = queue
        .reserve(&["default"], std::time::Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    queue.dead_letter(&reserved.job.id, "failed").await.unwrap();

    run(JobsCommand::Retry).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
}

#[cfg(feature = "jobs-db")]
#[tokio::test]
async fn jobs_failed_lists_dead_lettered_jobs_in_db_backend() {
    use doido_jobs::JobPayload;
    use serde_json::json;

    let _guard = CWD_LOCK.lock().unwrap();
    let _pool = doido_model::pool::test_lock();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "database:\n  url: \"sqlite::memory:\"\njobs:\n  type: db\n  queues: [default]\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();
    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");

    let cfg = doido_jobs::config::load();
    let queue = doido_jobs::config::build_configured_queue(&cfg)
        .await
        .unwrap();
    let job = JobPayload::new("default", json!({}), 0);
    queue.enqueue(job).await.unwrap();
    let reserved = queue
        .reserve(&["default"], std::time::Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    queue.dead_letter(&reserved.job.id, "boom").await.unwrap();

    run(JobsCommand::Failed).await;

    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }
}
