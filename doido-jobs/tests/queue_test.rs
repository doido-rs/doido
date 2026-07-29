use doido_jobs::queue::{JobPayload, JobStatus};
use serde_json::json;

#[test]
fn job_status_variants_are_distinct() {
    use doido_jobs::queue::JobStatus;
    assert_ne!(JobStatus::Pending, JobStatus::Running);
    assert_ne!(JobStatus::Dead, JobStatus::Done);
}

#[test]
fn payload_carries_default_timeout_and_backoff_base() {
    let j = JobPayload::new("default", json!({}), 1);
    assert_eq!(j.timeout, 30);
    assert_eq!(j.backoff_base, 5);
}

#[test]
fn test_job_payload_new_has_pending_status() {
    let j = JobPayload::new("default", json!({"user_id": 1}), 3);
    assert_eq!(j.status, JobStatus::Pending);
    assert_eq!(j.attempts, 0);
    assert_eq!(j.max_retries, 3);
}

#[test]
fn test_job_payload_has_unique_id() {
    let a = JobPayload::new("default", json!({}), 0);
    let b = JobPayload::new("default", json!({}), 0);
    assert_ne!(a.id, b.id);
}

#[test]
fn test_job_payload_fluent_setters() {
    use chrono::Utc;
    use doido_jobs::BackoffStrategy;

    let at = Utc::now() + chrono::Duration::hours(1);
    let j = JobPayload::new("default", json!({}), 2)
        .with_name("my_job")
        .with_backoff(BackoffStrategy::Linear, 10)
        .with_timeout(99)
        .with_run_at(at);

    assert_eq!(j.job_name, "my_job");
    assert_eq!(j.backoff, BackoffStrategy::Linear);
    assert_eq!(j.backoff_base, 10);
    assert_eq!(j.timeout, 99);
    assert_eq!(j.run_at, at);
    assert!(!j.is_ready(Utc::now()));
    assert!(j.is_ready(at));
}
