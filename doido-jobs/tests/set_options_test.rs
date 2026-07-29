use chrono::Utc;
use doido_jobs::JobPayload;
use serde_json::json;

#[test]
fn fluent_set_options_including_backoff_and_timeout() {
    use doido_jobs::BackoffStrategy;

    let job = JobPayload::new("default", json!({}), 3)
        .with_queue("mailers")
        .with_priority(5)
        .with_wait(60)
        .with_backoff(BackoffStrategy::Linear, 2)
        .with_timeout(15);

    assert_eq!(job.queue, "mailers");
    assert_eq!(job.backoff, BackoffStrategy::Linear);
    assert_eq!(job.backoff_base, 2);
    assert_eq!(job.timeout, 15);
}

#[test]
fn fluent_set_options() {
    let job = JobPayload::new("default", json!({}), 3)
        .with_queue("mailers")
        .with_priority(5)
        .with_wait(60);

    assert_eq!(job.queue, "mailers");
    assert_eq!(job.priority, 5);
    assert!(
        job.run_at > Utc::now(),
        "wait pushed run_at into the future"
    );
}
