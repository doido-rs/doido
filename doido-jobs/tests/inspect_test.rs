use chrono::{Duration, Utc};
use doido_jobs::inspect::JobBucket;
use doido_jobs::queue::{JobPayload, JobQueue, JobStatus};
use doido_jobs::MemoryQueue;
use serde_json::json;
use std::time::Duration as StdDuration;

#[tokio::test]
async fn memory_inspect_buckets_and_dead_admin() {
    let q = MemoryQueue::new();
    let mut pending = JobPayload::new("default", json!({"a": 1}), 3);
    pending.job_name = "work".into();
    q.enqueue(pending).await.unwrap();

    let mut scheduled = JobPayload::new("default", json!({}), 3);
    scheduled.run_at = Utc::now() + Duration::hours(1);
    q.enqueue(scheduled).await.unwrap();

    let counts = q.queue_counts(&["default"]).await.unwrap();
    assert_eq!(counts[0].1.pending_ready, 1);
    assert_eq!(counts[0].1.scheduled, 1);

    let ready = q
        .list_jobs("default", JobBucket::PendingReady, 10, None, false)
        .await
        .unwrap();
    assert_eq!(ready.jobs.len(), 1);

    let reserved = q
        .reserve(&["default"], StdDuration::ZERO)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        q.list_jobs("default", JobBucket::Running, 10, None, false)
            .await
            .unwrap()
            .jobs
            .len(),
        1
    );

    q.dead_letter(&reserved.job.id, "boom").await.unwrap();
    assert_eq!(
        q.list_jobs("default", JobBucket::Dead, 10, None, false)
            .await
            .unwrap()
            .jobs
            .len(),
        1
    );

    let dead_id = q.dead_jobs("default").await.unwrap()[0].id.clone();
    q.retry_dead_job("default", &dead_id).await.unwrap();
    assert!(q.dead_jobs("default").await.unwrap().is_empty());
    assert!(
        q.list_jobs("default", JobBucket::PendingReady, 10, None, false)
            .await
            .unwrap()
            .jobs
            .len()
            >= 1
    );

    let reserved2 = q
        .reserve(&["default"], StdDuration::ZERO)
        .await
        .unwrap()
        .unwrap();
    q.dead_letter(&reserved2.job.id, "again").await.unwrap();
    let dead_id2 = q.dead_jobs("default").await.unwrap()[0].id.clone();
    assert!(q.discard_dead_job("default", &dead_id2).await.unwrap());
    assert!(q.dead_jobs("default").await.unwrap().is_empty());
}

#[tokio::test]
async fn memory_failed_ready_bucket() {
    let q = MemoryQueue::new();
    let id = q
        .enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    let r = q
        .reserve(&["default"], StdDuration::ZERO)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.id, id);
    q.nack(&id, Some(Utc::now()), "err").await.unwrap();
    let ready = q
        .list_jobs("default", JobBucket::FailedReady, 10, None, false)
        .await
        .unwrap();
    assert_eq!(ready.jobs.len(), 1);
}

#[tokio::test]
async fn memory_failed_retry_bucket() {
    let q = MemoryQueue::new();
    let id = q
        .enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    let r = q
        .reserve(&["default"], StdDuration::ZERO)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.id, id);
    let later = Utc::now() + Duration::minutes(5);
    q.nack(&id, Some(later), "err").await.unwrap();
    let retry = q
        .list_jobs("default", JobBucket::Retry, 10, None, false)
        .await
        .unwrap();
    assert_eq!(retry.jobs.len(), 1);
    assert_eq!(retry.jobs[0].status, JobStatus::Failed);
}

#[tokio::test]
async fn memory_list_jobs_cursor_pages_and_in_queue() {
    use doido_jobs::inspect::decode_job_page_token;

    let q = MemoryQueue::new();
    for i in 0..6 {
        let mut j = JobPayload::new("default", json!({ "i": i }), 3);
        j.id = format!("job-{i}");
        j.priority = i;
        j.run_at = Utc::now() - Duration::hours(1);
        q.enqueue(j).await.unwrap();
    }

    let all = q
        .list_jobs("default", JobBucket::PendingReady, 20, None, false)
        .await
        .unwrap();
    assert_eq!(
        all.jobs.len(),
        6,
        "expected every enqueued job to be pending-ready"
    );

    let p1 = q
        .list_jobs("default", JobBucket::PendingReady, 2, None, false)
        .await
        .unwrap();
    assert_eq!(p1.jobs.len(), 2);
    assert!(p1.next_page_token.is_some());
    let anchor = decode_job_page_token(p1.next_page_token.as_ref().unwrap()).unwrap();
    let p2 = q
        .list_jobs("default", JobBucket::PendingReady, 2, Some(anchor), false)
        .await
        .unwrap();
    assert!(!p2.jobs.is_empty());
    assert_ne!(p2.jobs[0].id, p1.jobs[0].id);
    assert!(p2.prev_page_token.is_some());
    if let Some(prev) = p2
        .prev_page_token
        .as_deref()
        .and_then(decode_job_page_token)
    {
        let back = q
            .list_jobs("default", JobBucket::PendingReady, 2, Some(prev), false)
            .await
            .unwrap();
        assert!(!back.jobs.is_empty());
    }

    let in_q = q
        .list_jobs("default", JobBucket::InQueue, 10, None, false)
        .await
        .unwrap();
    assert_eq!(in_q.jobs.len(), 6);
}

#[cfg(feature = "jobs-db")]
mod db {
    use super::*;
    use doido_jobs::inspect::decode_job_page_token;
    use doido_jobs::DbQueue;
    use doido_model::sea_orm::{ConnectOptions, Database};

    async fn sqlite_queue() -> DbQueue {
        let mut opts = ConnectOptions::new("sqlite::memory:".to_string());
        opts.sqlx_logging(false);
        let conn = Database::connect(opts).await.unwrap();
        let q = DbQueue::new(conn);
        q.migrate().await.unwrap();
        q
    }

    #[tokio::test]
    async fn db_queue_counts_and_list_buckets() {
        let q = sqlite_queue().await;
        let mut scheduled = JobPayload::new("default", json!({}), 3);
        scheduled.run_at = Utc::now() + Duration::hours(2);
        q.enqueue(scheduled).await.unwrap();
        q.enqueue(JobPayload::new("default", json!({}), 3))
            .await
            .unwrap();

        let counts = q.queue_counts(&["default"]).await.unwrap();
        assert_eq!(counts[0].1.pending_ready, 1);
        assert_eq!(counts[0].1.scheduled, 1);

        assert_eq!(
            q.list_jobs("default", JobBucket::Scheduled, 10, None, false)
                .await
                .unwrap()
                .jobs
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn db_retry_dead_job() {
        let q = sqlite_queue().await;
        let id = q
            .enqueue(JobPayload::new("default", json!({}), 0))
            .await
            .unwrap();
        let r = q
            .reserve(&["default"], StdDuration::ZERO)
            .await
            .unwrap()
            .unwrap();
        q.dead_letter(&r.job.id, "x").await.unwrap();
        q.retry_dead_job("default", &id).await.unwrap();
        assert!(q.dead_jobs("default").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn db_inspect_and_discard_dead_job() {
        let q = sqlite_queue().await;
        let id = q
            .enqueue(JobPayload::new("default", json!({}), 0))
            .await
            .unwrap();
        let r = q
            .reserve(&["default"], StdDuration::ZERO)
            .await
            .unwrap()
            .unwrap();
        q.dead_letter(&r.job.id, "fatal").await.unwrap();
        assert_eq!(
            q.list_jobs("default", JobBucket::Dead, 10, None, false)
                .await
                .unwrap()
                .jobs
                .len(),
            1
        );
        assert!(q.discard_dead_job("default", &id).await.unwrap());
        assert!(q
            .list_jobs("default", JobBucket::Dead, 10, None, false)
            .await
            .unwrap()
            .jobs
            .is_empty());
    }

    #[tokio::test]
    async fn db_aggregate_queue_counts_across_queues() {
        let q = sqlite_queue().await;
        q.enqueue(JobPayload::new("alpha", json!({}), 3))
            .await
            .unwrap();
        q.enqueue(JobPayload::new("beta", json!({}), 3))
            .await
            .unwrap();
        let mut scheduled = JobPayload::new("gamma", json!({}), 3);
        scheduled.run_at = Utc::now() + Duration::hours(1);
        q.enqueue(scheduled).await.unwrap();

        let counts = q
            .queue_counts(&["alpha", "beta", "gamma", "empty"])
            .await
            .unwrap();
        assert_eq!(counts.len(), 4);
        let by_name: std::collections::HashMap<_, _> = counts.into_iter().collect();
        assert_eq!(by_name["alpha"].pending_ready, 1);
        assert_eq!(by_name["beta"].pending_ready, 1);
        assert_eq!(by_name["gamma"].scheduled, 1);
        assert_eq!(by_name["empty"].pending_ready, 0);
    }

    #[tokio::test]
    async fn db_list_jobs_cursor_pages_and_in_queue() {
        let q = sqlite_queue().await;
        for i in 0..5 {
            let mut j = JobPayload::new("default", json!({}), 3);
            j.id = format!("db-job-{i}");
            j.priority = i;
            q.enqueue(j).await.unwrap();
        }

        let p1 = q
            .list_jobs("default", JobBucket::PendingReady, 2, None, false)
            .await
            .unwrap();
        assert_eq!(p1.jobs.len(), 2);
        let anchor = decode_job_page_token(p1.next_page_token.as_ref().unwrap()).unwrap();
        let p2 = q
            .list_jobs("default", JobBucket::PendingReady, 2, Some(anchor), false)
            .await
            .unwrap();
        assert_eq!(p2.jobs.len(), 2);

        let in_q = q
            .list_jobs("default", JobBucket::InQueue, 10, None, false)
            .await
            .unwrap();
        assert_eq!(in_q.jobs.len(), 5);
    }
}
