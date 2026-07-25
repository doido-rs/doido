use chrono::Utc;
use doido_jobs::JobPayload;
use serde_json::json;

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
