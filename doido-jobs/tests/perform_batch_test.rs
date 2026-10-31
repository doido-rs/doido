//! Batch enqueue / perform helpers.

use doido_jobs::{
    enqueue_jobs, perform_jobs_now, JobContext, JobPayload, JobQueue, JobRegistry, MemoryQueue,
};
use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

static BATCH_LOCK: Mutex<()> = Mutex::new(());
static BATCH_SUM: AtomicU32 = AtomicU32::new(0);

#[doido_jobs::job]
async fn batch_add(n: u32) -> doido_core::Result<()> {
    BATCH_SUM.fetch_add(n, Ordering::SeqCst);
    Ok(())
}

#[tokio::test]
async fn enqueue_jobs_enqueues_all() {
    let _guard = BATCH_LOCK.lock().unwrap();
    let queue = Arc::new(MemoryQueue::new());
    let jobs = vec![
        JobPayload::new("default", json!(1), 3).with_name("batch_add"),
        JobPayload::new("default", json!(2), 3).with_name("batch_add"),
    ];
    let ids = enqueue_jobs(queue.as_ref(), jobs).await.unwrap();
    assert_eq!(ids.len(), 2);
    let page = queue
        .list_jobs("default", doido_jobs::JobBucket::InQueue, 10, None, true)
        .await
        .unwrap();
    assert_eq!(page.jobs.len(), 2);
}

#[tokio::test]
async fn perform_jobs_now_runs_sequentially() {
    let _guard = BATCH_LOCK.lock().unwrap();
    BATCH_SUM.store(0, Ordering::SeqCst);
    let registry = JobRegistry::from_inventory();
    let jobs = vec![
        JobPayload::new("default", json!(4), 0)
            .with_name("batch_add")
            .with_timeout(30),
        JobPayload::new("default", json!(5), 0)
            .with_name("batch_add")
            .with_timeout(30),
    ];
    perform_jobs_now(&registry, jobs, Arc::new(JobContext::new()))
        .await
        .unwrap();
    assert_eq!(BATCH_SUM.load(Ordering::SeqCst), 9);
}

#[tokio::test]
async fn enqueue_many_generated_helper() {
    let _guard = BATCH_LOCK.lock().unwrap();
    let queue = Arc::new(MemoryQueue::new());
    batch_add_enqueue_many(queue.as_ref(), [10u32, 11])
        .await
        .unwrap();
    let page = queue
        .list_jobs("default", doido_jobs::JobBucket::InQueue, 10, None, true)
        .await
        .unwrap();
    assert_eq!(page.jobs.len(), 2);
}
