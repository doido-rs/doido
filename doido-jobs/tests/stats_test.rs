use doido_jobs::{JobPayload, JobQueue, MemoryQueue};
use serde_json::json;

#[tokio::test]
async fn stats_report_pending_counts_per_queue() {
    let q = MemoryQueue::new();
    q.enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    q.enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    q.enqueue(JobPayload::new("mailers", json!({}), 3))
        .await
        .unwrap();

    let default = q.stats("default").await;
    assert_eq!(default.pending, 2);
    assert_eq!(default.dead, 0);

    assert_eq!(q.stats("mailers").await.pending, 1);
    assert_eq!(q.stats("empty").await.pending, 0);
}

#[tokio::test]
async fn stats_include_dead_letter_count() {
    let q = MemoryQueue::new();
    let job = JobPayload::new("default", json!({}), 1);
    q.enqueue(job).await.unwrap();
    let r = q
        .reserve(&["default"], std::time::Duration::from_millis(50))
        .await
        .unwrap()
        .unwrap();
    q.dead_letter(&r.job.id, "fatal").await.unwrap();
    let stats = q.stats("default").await;
    assert_eq!(stats.dead, 1);
    assert_eq!(stats.pending, 0);
}
