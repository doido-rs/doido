#![cfg(feature = "jobs-db")]

use chrono::{Duration as ChronoDuration, Utc};
use doido_jobs::db::DbQueue;
use doido_jobs::queue::{JobPayload, JobQueue, JobStatus};
use doido_jobs::{EngineConfig, WorkerEngine};
use doido_model::sea_orm::Database;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

const QUEUES: &[&str] = &["default"];

async fn fresh() -> DbQueue {
    let conn = Database::connect("sqlite::memory:").await.unwrap();
    let q = DbQueue::new(conn);
    q.migrate().await.unwrap();
    q
}

#[tokio::test]
async fn test_db_enqueue_reserve_ack() {
    let q = fresh().await;
    q.enqueue(JobPayload::new("default", json!({"x": 1}), 3))
        .await
        .unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.status, JobStatus::Running);
    assert_eq!(r.job.attempts, 1);
    q.ack(&r.job.id).await.unwrap();
    assert!(q
        .reserve(QUEUES, Duration::from_millis(20))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_db_priority_order() {
    let q = fresh().await;
    q.enqueue(JobPayload::new("default", json!({"p": "low"}), 3).with_priority(0))
        .await
        .unwrap();
    q.enqueue(JobPayload::new("default", json!({"p": "high"}), 3).with_priority(10))
        .await
        .unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.payload["p"], "high");
}

#[tokio::test]
async fn test_db_enqueue_at_defers() {
    let q = fresh().await;
    let at = Utc::now() + ChronoDuration::seconds(60);
    q.enqueue_at(JobPayload::new("default", json!({}), 3), at)
        .await
        .unwrap();
    assert!(q
        .reserve(QUEUES, Duration::from_millis(30))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_db_nack_and_dead_letter() {
    let q = fresh().await;
    q.enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .unwrap();
    q.nack(&r.job.id, None, "boom").await.unwrap();
    let r2 = q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r2.job.attempts, 2);
    q.dead_letter(&r2.job.id, "fatal").await.unwrap();
    let dead = q.dead_jobs("default").await.unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].status, JobStatus::Dead);
    assert_eq!(dead[0].error.as_deref(), Some("fatal"));
}

#[tokio::test]
async fn test_db_reclaim_expired() {
    let conn = Database::connect("sqlite::memory:").await.unwrap();
    let q = DbQueue::new(conn).with_visibility_timeout(Duration::from_millis(0));
    q.migrate().await.unwrap();
    q.enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    let _r = q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .unwrap();
    let reclaimed = q.reclaim_expired(QUEUES).await.unwrap();
    assert_eq!(reclaimed, 1);
    assert!(q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_db_discard_dead() {
    let q = fresh().await;
    q.enqueue(JobPayload::new("default", json!({}), 1))
        .await
        .unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .unwrap();
    q.dead_letter(&r.job.id, "fatal").await.unwrap();
    assert_eq!(q.discard_dead("default").await.unwrap(), 1);
    assert!(q.dead_jobs("default").await.unwrap().is_empty());
}

#[tokio::test]
async fn test_db_reclaim_empty_queues_is_noop() {
    let q = fresh().await;
    assert_eq!(q.reclaim_expired(&[]).await.unwrap(), 0);
}

#[tokio::test]
async fn test_db_nack_missing_job_is_ok() {
    let q = fresh().await;
    q.nack("missing-id", None, "err").await.unwrap();
}

#[tokio::test]
async fn test_db_reserve_empty_queues() {
    let q = fresh().await;
    assert!(q
        .reserve(&[], Duration::from_millis(10))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_db_ack_missing_job_is_ok() {
    let q = fresh().await;
    q.ack("missing").await.unwrap();
}

#[tokio::test]
async fn test_db_reclaim_skips_other_queues() {
    let q = fresh().await;
    q.enqueue(JobPayload::new("other", json!({}), 1))
        .await
        .unwrap();
    let _ = q
        .reserve(&["other"], Duration::from_millis(100))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(q.reclaim_expired(&["default"]).await.unwrap(), 0);
}

#[tokio::test]
async fn test_db_dead_letter_missing_job_is_ok() {
    let q = fresh().await;
    q.dead_letter("missing", "reason").await.unwrap();
}

#[tokio::test]
async fn test_db_engine_panic_keeps_row_as_dead() {
    let q = Arc::new(fresh().await);
    q.enqueue(JobPayload::new("default", json!({}), 0))
        .await
        .unwrap();
    let engine = WorkerEngine::new(
        q.clone(),
        EngineConfig {
            queues: vec!["default".into()],
            concurrency: 1,
            poll_wait: Duration::from_millis(50),
            reclaim_interval: Duration::from_secs(60),
        },
    );
    engine
        .run_once(&|_job, _ctx| async { panic!("db panic test") })
        .await
        .unwrap();
    let dead = q.dead_jobs("default").await.unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].error.as_deref(), Some("job panicked"));
}

#[tokio::test]
async fn test_db_reclaim_syncs_json_then_run_once_processes() {
    let conn = Database::connect("sqlite::memory:").await.unwrap();
    let q = Arc::new(DbQueue::new(conn).with_visibility_timeout(Duration::from_millis(0)));
    q.migrate().await.unwrap();
    q.enqueue(JobPayload::new("default", json!({"n": 1}), 3))
        .await
        .unwrap();
    let reserved = q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .expect("job reserved");
    assert_eq!(reserved.job.status, JobStatus::Running);

    assert_eq!(q.reclaim_expired(QUEUES).await.unwrap(), 1);

    let engine = WorkerEngine::new(
        q.clone(),
        EngineConfig {
            queues: vec!["default".into()],
            concurrency: 1,
            poll_wait: Duration::from_millis(50),
            reclaim_interval: Duration::from_secs(60),
        },
    );
    let seen = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = seen.clone();
    engine
        .run_once(&move |_job, _ctx| {
            let flag = flag.clone();
            async move {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        })
        .await
        .unwrap();
    assert!(seen.load(std::sync::atomic::Ordering::SeqCst));
}

#[tokio::test]
async fn test_db_reclaim_sets_lease_expired_error() {
    let conn = Database::connect("sqlite::memory:").await.unwrap();
    let q = DbQueue::new(conn).with_visibility_timeout(Duration::from_millis(0));
    q.migrate().await.unwrap();
    q.enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    let _ = q.reserve(QUEUES, Duration::from_millis(100)).await.unwrap();
    q.reclaim_expired(QUEUES).await.unwrap();
    let r = q
        .reserve(QUEUES, Duration::from_millis(100))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.error.as_deref(), Some("lease expired"));
}
