use doido_jobs::memory::MemoryQueue;
use doido_jobs::queue::{JobPayload, JobQueue, JobStatus};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_enqueue_and_dequeue() {
    let q = Arc::new(MemoryQueue::new());
    let job = JobPayload::new("default", json!({"x": 1}), 3);
    q.enqueue(job).await.unwrap();
    let dequeued = q.dequeue("default").await.unwrap();
    assert!(dequeued.is_some());
    let j = dequeued.unwrap();
    assert_eq!(j.status, JobStatus::Running);
    assert_eq!(j.attempts, 1);
}

#[tokio::test]
async fn test_ack_removes_from_running() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 3);
    q.enqueue(job).await.unwrap();
    let j = q.dequeue("default").await.unwrap().unwrap();
    q.ack(&j.id).await.unwrap();
    // dequeue again — should be empty
    assert!(q.dequeue("default").await.unwrap().is_none());
}

#[tokio::test]
async fn test_nack_re_enqueues_if_retries_remain() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 3);
    q.enqueue(job).await.unwrap();
    let j = q.dequeue("default").await.unwrap().unwrap();
    q.nack(&j.id, "transient error").await.unwrap();
    // should be back in queue
    let j2 = q.dequeue("default").await.unwrap();
    assert!(j2.is_some());
    assert_eq!(j2.unwrap().attempts, 2);
}

#[tokio::test]
async fn test_nack_moves_to_dead_after_max_retries() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 1);
    q.enqueue(job).await.unwrap();
    let j = q.dequeue("default").await.unwrap().unwrap();
    // attempts is now 1, max_retries is 1 → should go dead
    q.nack(&j.id, "fatal").await.unwrap();
    let dead = q.dead_jobs("default").await.unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].status, JobStatus::Dead);
}
