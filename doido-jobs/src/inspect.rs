//! Queue introspection for dashboards (`JobQueue` list/count APIs).

mod page;

pub use page::{
    decode_job_page_token, decode_queue_page_token, encode_job_page_token, encode_queue_page_token,
    encode_queue_page_token_before, job_sort_cmp, paginate_queue_names, paginate_sorted,
    JobListPage, JobPageAnchor, JobPageDirection, JobSortKey, QueueListPage, QueuePageAnchor,
};

use crate::queue::{JobPayload, JobStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Per-queue job counts for the dashboard **Queues** tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueCounts {
    pub pending_ready: u32,
    pub scheduled: u32,
    pub running: u32,
    pub retry: u32,
    /// Failed jobs eligible to be reserved again (`run_at <= now`).
    pub failed_ready: u32,
    pub dead: u32,
}

impl QueueCounts {
    pub fn total_waiting(&self) -> u32 {
        self.pending_ready
            .saturating_add(self.scheduled)
            .saturating_add(self.retry)
            .saturating_add(self.failed_ready)
    }
}

/// Logical bucket for the dashboard tabs (except the aggregate **Queues** tab).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobBucket {
    /// Pending and eligible now (`run_at <= now`).
    PendingReady,
    /// Pending but scheduled for the future.
    Scheduled,
    /// Currently leased / running.
    Running,
    /// Failed, waiting for backoff (`run_at > now`).
    Retry,
    /// Failed and eligible to run again (`run_at <= now`).
    FailedReady,
    /// Dead-letter store.
    Dead,
    /// Dashboard **In queue** tab: [`PendingReady`] + [`FailedReady`].
    InQueue,
}

/// Classify a job sitting in a backend's pending/failed store (not running/dead).
pub fn classify_stored(job: &JobPayload, now: DateTime<Utc>) -> Option<JobBucket> {
    match job.status {
        JobStatus::Pending if job.is_ready(now) => Some(JobBucket::PendingReady),
        JobStatus::Pending => Some(JobBucket::Scheduled),
        JobStatus::Failed if job.is_ready(now) => Some(JobBucket::FailedReady),
        JobStatus::Failed => Some(JobBucket::Retry),
        JobStatus::Running => Some(JobBucket::Running),
        JobStatus::Dead => Some(JobBucket::Dead),
        JobStatus::Done => None,
    }
}

/// Update [`QueueCounts`] for a stored (non-running) job.
pub fn count_stored(counts: &mut QueueCounts, job: &JobPayload, now: DateTime<Utc>) {
    match classify_stored(job, now) {
        Some(JobBucket::PendingReady) => counts.pending_ready += 1,
        Some(JobBucket::Scheduled) => counts.scheduled += 1,
        Some(JobBucket::Retry) => counts.retry += 1,
        Some(JobBucket::FailedReady) => counts.failed_ready += 1,
        Some(JobBucket::Dead) => counts.dead += 1,
        Some(JobBucket::Running) => counts.running += 1,
        Some(JobBucket::InQueue) | None => {}
    }
}

pub fn matches_bucket(job: &JobPayload, bucket: JobBucket, now: DateTime<Utc>) -> bool {
    classify_stored(job, now) == Some(bucket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::JobStatus;
    use serde_json::json;

    #[test]
    fn classify_pending_ready_vs_scheduled() {
        let now = Utc::now();
        let mut ready = JobPayload::new("q", json!({}), 1);
        ready.status = JobStatus::Pending;
        ready.run_at = now;
        assert_eq!(classify_stored(&ready, now), Some(JobBucket::PendingReady));

        let mut later = ready.clone();
        later.run_at = now + chrono::Duration::hours(1);
        assert_eq!(classify_stored(&later, now), Some(JobBucket::Scheduled));
    }

    #[test]
    fn queue_counts_total_waiting_sums_buckets() {
        let c = QueueCounts {
            pending_ready: 1,
            scheduled: 2,
            running: 0,
            retry: 3,
            failed_ready: 4,
            dead: 0,
        };
        assert_eq!(c.total_waiting(), 10);
    }
}
