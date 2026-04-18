use doido_jobs::queue::{JobPayload, JobStatus};
use serde_json::json;

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
