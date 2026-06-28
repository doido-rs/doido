#![cfg(feature = "jobs-redis")]

//! These tests require a reachable Redis. They self-skip when `REDIS_URL` is unset
//! or the server is unreachable, so the suite stays green in CI without Redis.

use doido_jobs::queue::{JobPayload, JobQueue, JobStatus};
use doido_jobs::redis::RedisQueue;
use serde_json::json;
use std::time::Duration;

const QUEUES: &[&str] = &["default"];

async fn queue() -> Option<RedisQueue> {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    // Unique namespace per run to avoid cross-test contamination.
    let ns = format!("doido:jobs:test:{}", uuid::Uuid::new_v4());
    match RedisQueue::connect(&url, ns).await {
        Ok(q) => {
            // Confirm the server actually answers before running assertions.
            if q.reserve(QUEUES, Duration::from_millis(10)).await.is_err() {
                return None;
            }
            Some(q)
        }
        Err(_) => None,
    }
}

#[tokio::test]
async fn test_redis_enqueue_reserve_ack() {
    let Some(q) = queue().await else {
        eprintln!("skipping: no Redis available");
        return;
    };
    q.enqueue(JobPayload::new("default", json!({"x": 1}), 3))
        .await
        .unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(200))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.status, JobStatus::Running);
    assert_eq!(r.job.attempts, 1);
    q.ack(&r.job.id).await.unwrap();
    assert!(q
        .reserve(QUEUES, Duration::from_millis(50))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_redis_priority_and_dead_letter() {
    let Some(q) = queue().await else {
        eprintln!("skipping: no Redis available");
        return;
    };
    q.enqueue(JobPayload::new("default", json!({"p": "low"}), 1).with_priority(0))
        .await
        .unwrap();
    q.enqueue(JobPayload::new("default", json!({"p": "high"}), 1).with_priority(10))
        .await
        .unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(200))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.payload["p"], "high");
    q.dead_letter(&r.job.id, "fatal").await.unwrap();
    let dead = q.dead_jobs("default").await.unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].status, JobStatus::Dead);
}

#[tokio::test]
async fn test_redis_reclaim_expired() {
    let url = match std::env::var("REDIS_URL") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("skipping: no Redis available");
            return;
        }
    };
    let ns = format!("doido:jobs:test:{}", uuid::Uuid::new_v4());
    let Ok(q) = RedisQueue::connect(&url, ns).await else {
        eprintln!("skipping: no Redis available");
        return;
    };
    let q = q.with_visibility_timeout(Duration::from_millis(0));
    if q.enqueue(JobPayload::new("default", json!({}), 3)).await.is_err() {
        eprintln!("skipping: no Redis available");
        return;
    }
    let _r = q
        .reserve(QUEUES, Duration::from_millis(200))
        .await
        .unwrap()
        .unwrap();
    let reclaimed = q.reclaim_expired(QUEUES).await.unwrap();
    assert_eq!(reclaimed, 1);
}
