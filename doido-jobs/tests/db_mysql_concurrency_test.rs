//! Parallel reserve against MySQL. Set `DOIDO_JOBS_MYSQL_URL` (e.g. from docker compose)
//! to run; otherwise tests exit early and pass in CI.

#![cfg(all(feature = "jobs-db", feature = "mysql"))]

use doido_jobs::db::DbQueue;
use doido_jobs::queue::{JobPayload, JobQueue, JobStatus};
use doido_model::sea_orm::Database;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

const QUEUES: &[&str] = &["default"];
const PARALLEL_RESERVES: usize = 4;
const JOB_COUNT: usize = 20;

async fn mysql_queue() -> Option<DbQueue> {
    let url = std::env::var("DOIDO_JOBS_MYSQL_URL").ok()?;
    let conn = Database::connect(&url).await.ok()?;
    let q = DbQueue::new(conn);
    q.migrate().await.ok()?;
    Some(q)
}

#[tokio::test]
async fn mysql_parallel_reserve_claims_distinct_jobs() {
    let Some(q) = mysql_queue().await else {
        eprintln!("skip: DOIDO_JOBS_MYSQL_URL not set or connect/migrate failed");
        return;
    };
    let q = Arc::new(q);

    for i in 0..JOB_COUNT {
        q.enqueue(JobPayload::new("default", json!({"i": i}), 3))
            .await
            .expect("enqueue");
    }

    let mut handles = Vec::with_capacity(PARALLEL_RESERVES);
    for _ in 0..PARALLEL_RESERVES {
        let q = Arc::clone(&q);
        handles.push(tokio::spawn(async move {
            q.reserve(QUEUES, Duration::from_secs(5))
                .await
                .expect("reserve")
        }));
    }

    let mut ids = HashSet::new();
    for handle in handles {
        let reserved = handle.await.expect("join").expect("expected a job");
        assert_eq!(reserved.job.status, JobStatus::Running);
        assert!(ids.insert(reserved.job.id));
    }
    assert_eq!(ids.len(), PARALLEL_RESERVES);
}
