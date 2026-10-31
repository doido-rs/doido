//! In-process job execution (`perform_job_now`) and CLI run helpers.

use doido_jobs::{
    perform_job_now, suggest_job_names, JobContext, JobPayload, JobRegistry, MemoryQueue,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static PERFORM_TEST_LOCK: Mutex<()> = Mutex::new(());
static PERFORM_RAN: AtomicU32 = AtomicU32::new(0);

#[doido_jobs::job(timeout = 1)]
async fn perform_test_job(n: u32) -> doido_core::Result<()> {
    if n == 999 {
        return Err(doido_core::anyhow::anyhow!("boom"));
    }
    if n == 888 {
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    PERFORM_RAN.fetch_add(n, Ordering::SeqCst);
    Ok(())
}

#[tokio::test]
async fn perform_job_now_runs_handler() {
    let _guard = PERFORM_TEST_LOCK.lock().unwrap();
    PERFORM_RAN.store(0, Ordering::SeqCst);
    let registry = JobRegistry::from_inventory();
    let job = JobPayload::new("default", serde_json::json!(3), 0)
        .with_name("perform_test_job")
        .with_timeout(30);
    perform_job_now(&registry, job, Arc::new(JobContext::new()))
        .await
        .unwrap();
    assert_eq!(PERFORM_RAN.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn perform_job_now_propagates_handler_error() {
    let _guard = PERFORM_TEST_LOCK.lock().unwrap();
    let registry = JobRegistry::from_inventory();
    let job = JobPayload::new("default", serde_json::json!(999), 0)
        .with_name("perform_test_job")
        .with_timeout(30);
    let err = perform_job_now(&registry, job, Arc::new(JobContext::new()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("boom"));
}

#[tokio::test]
async fn perform_job_now_times_out() {
    let _guard = PERFORM_TEST_LOCK.lock().unwrap();
    let registry = JobRegistry::from_inventory();
    let job = JobPayload::new("default", serde_json::json!(888), 0)
        .with_name("perform_test_job")
        .with_timeout(1);
    let err = perform_job_now(&registry, job, Arc::new(JobContext::new()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("timed out"));
}

#[test]
fn suggest_job_names_finds_prefix_match() {
    let known = ["sync_nfse_catalog_daily", "import_nfse_anexo_a"];
    let hints = suggest_job_names(&known, "sync_nfse", 3);
    assert_eq!(hints, vec!["sync_nfse_catalog_daily"]);
}

#[tokio::test]
async fn perform_now_does_not_touch_queue() {
    let _guard = PERFORM_TEST_LOCK.lock().unwrap();
    let memory = Arc::new(MemoryQueue::new());
    let before = memory.stats("default").await.pending;
    let registry = JobRegistry::from_inventory();
    let job = JobPayload::new("default", serde_json::json!(1), 0)
        .with_name("perform_test_job")
        .with_timeout(30);
    perform_job_now(&registry, job, Arc::new(JobContext::new()))
        .await
        .unwrap();
    let after = memory.stats("default").await.pending;
    assert_eq!(before, after);
}
