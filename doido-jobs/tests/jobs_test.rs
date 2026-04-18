use doido_jobs::{JobPayload, JobQueue, MemoryQueue, Worker};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_full_job_lifecycle_enqueue_perform_ack() {
    let queue = Arc::new(MemoryQueue::new());
    let job = JobPayload::new("default", json!({"task": "send_email"}), 3);
    queue.enqueue(job).await.unwrap();

    let worker = Worker::new(queue.clone(), "default");
    let performed = Arc::new(Mutex::new(false));
    let performed_clone = performed.clone();

    worker.run_once(|_job| {
        let p = performed_clone.clone();
        async move {
            *p.lock().await = true;
            Ok(())
        }
    }).await.unwrap();

    assert!(*performed.lock().await);
}

#[tokio::test]
async fn test_failed_job_goes_to_dead_after_max_retries() {
    let queue = Arc::new(MemoryQueue::new());
    let job = JobPayload::new("default", json!({}), 1);
    queue.enqueue(job).await.unwrap();

    let worker = Worker::new(queue.clone(), "default");
    worker.run_once(|_job| async {
        Err(doido_core::anyhow::anyhow!("always fails"))
    }).await.unwrap();

    let dead = queue.dead_jobs("default").await.unwrap();
    assert_eq!(dead.len(), 1);
}

#[tokio::test]
async fn test_job_macro_compiles() {
    use doido_jobs::job;

    #[job]
    async fn my_job(_data: serde_json::Value) -> doido_core::Result<()> {
        Ok(())
    }

    // just verifying the macro compiles and the function is callable
    my_job(json!({})).await.unwrap();
}
