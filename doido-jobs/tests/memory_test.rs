use chrono::{Duration as ChronoDuration, Utc};
use doido_jobs::memory::MemoryQueue;
use doido_jobs::queue::{JobPayload, JobQueue, JobStatus};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

const QUEUES: &[&str] = &["default"];

#[tokio::test]
async fn test_enqueue_and_reserve() {
    let q = Arc::new(MemoryQueue::new());
    let job = JobPayload::new("default", json!({"x": 1}), 3);
    q.enqueue(job).await.unwrap();
    let reserved = q.reserve(QUEUES, Duration::from_millis(50)).await.unwrap();
    assert!(reserved.is_some());
    let r = reserved.unwrap();
    assert_eq!(r.job.status, JobStatus::Running);
    assert_eq!(r.job.attempts, 1);
}

#[tokio::test]
async fn test_ack_removes_from_running() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 3);
    q.enqueue(job).await.unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    q.ack(&r.job.id).await.unwrap();
    // reserve again — should be empty
    assert!(q
        .reserve(QUEUES, Duration::from_millis(10))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_nack_re_enqueues_at_retry_at() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 3);
    q.enqueue(job).await.unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    // retry immediately
    q.nack(&r.job.id, None, "transient error").await.unwrap();
    let r2 = q.reserve(QUEUES, Duration::from_millis(50)).await.unwrap();
    assert!(r2.is_some());
    assert_eq!(r2.unwrap().job.attempts, 2);
}

#[tokio::test]
async fn test_nack_with_future_retry_at_is_not_immediately_ready() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 3);
    q.enqueue(job).await.unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    let future = Utc::now() + ChronoDuration::seconds(60);
    q.nack(&r.job.id, Some(future), "later").await.unwrap();
    // not yet due
    assert!(q
        .reserve(QUEUES, Duration::from_millis(20))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_dead_letter_moves_job() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 1);
    q.enqueue(job).await.unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    q.dead_letter(&r.job.id, "fatal").await.unwrap();
    let dead = q.dead_jobs("default").await.unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].status, JobStatus::Dead);
}

#[tokio::test]
async fn test_enqueue_at_defers_until_run_at() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 3);
    let at = Utc::now() + ChronoDuration::seconds(30);
    q.enqueue_at(job, at).await.unwrap();
    assert!(q
        .reserve(QUEUES, Duration::from_millis(20))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_priority_order_within_queue() {
    let q = MemoryQueue::new();
    q.enqueue(JobPayload::new("default", json!({"p": "low"}), 3).with_priority(0))
        .await
        .unwrap();
    q.enqueue(JobPayload::new("default", json!({"p": "high"}), 3).with_priority(10))
        .await
        .unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.payload["p"], "high");
}

#[tokio::test]
async fn test_reclaim_expired_returns_lease_to_queue() {
    let q = MemoryQueue::new().with_visibility_timeout(Duration::from_millis(0));
    let job = JobPayload::new("default", json!({}), 3);
    q.enqueue(job).await.unwrap();
    let _r = q
        .reserve(QUEUES, Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    // lease already expired (vt = 0)
    let reclaimed = q.reclaim_expired(QUEUES).await.unwrap();
    assert_eq!(reclaimed, 1);
    // job is reservable again
    assert!(q
        .reserve(QUEUES, Duration::from_millis(50))
        .await
        .unwrap()
        .is_some());
}
