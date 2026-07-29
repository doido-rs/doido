use chrono::{DateTime, Utc};
use doido_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Unique identifier of an enqueued job.
pub type JobId = String;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Dead,
}

/// Retry pacing strategy. The engine uses this to compute the next `retry_at`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BackoffStrategy {
    #[default]
    Exponential,
    Linear,
    None,
}

impl BackoffStrategy {
    /// Delay before the next attempt. `attempt` is the 1-based attempt that just failed.
    pub fn delay(&self, attempt: u32, base_secs: u64) -> Duration {
        let secs = match self {
            BackoffStrategy::Exponential => {
                let exp = attempt.saturating_sub(1).min(32);
                base_secs.saturating_mul(1u64 << exp)
            }
            BackoffStrategy::Linear => base_secs.saturating_mul(attempt as u64),
            BackoffStrategy::None => 0,
        };
        Duration::from_secs(secs)
    }
}

fn default_run_at() -> DateTime<Utc> {
    Utc::now()
}

fn default_backoff_base() -> u64 {
    5
}

fn default_timeout() -> u64 {
    30
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobPayload {
    pub id: JobId,
    pub queue: String,
    /// The registered name of the job type, used by the worker to look up the
    /// handler that runs this payload (the `#[job]` macro stamps the job's
    /// function name here). Empty for anonymous/raw payloads. `#[serde(default)]`
    /// keeps already-persisted jobs (written before this field existed) loadable.
    #[serde(default)]
    pub job_name: String,
    pub payload: Value,
    pub attempts: u32,
    pub max_retries: u32,
    pub status: JobStatus,
    pub error: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_run_at")]
    pub run_at: DateTime<Utc>,
    #[serde(default)]
    pub backoff: BackoffStrategy,
    #[serde(default = "default_backoff_base")]
    pub backoff_base: u64,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl JobPayload {
    pub fn new(queue: impl Into<String>, payload: Value, max_retries: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            queue: queue.into(),
            job_name: String::new(),
            payload,
            attempts: 0,
            max_retries,
            status: JobStatus::Pending,
            error: None,
            priority: 0,
            run_at: Utc::now(),
            backoff: BackoffStrategy::default(),
            backoff_base: default_backoff_base(),
            timeout: default_timeout(),
        }
    }

    /// Stamp the registered job-type name (the `#[job]` macro sets this to the
    /// job's function name) so the worker can route the payload to its handler.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.job_name = name.into();
        self
    }

    /// Fluent setters used by the `#[job]` macro to stamp policy onto the payload.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_backoff(mut self, backoff: BackoffStrategy, base_secs: u64) -> Self {
        self.backoff = backoff;
        self.backoff_base = base_secs;
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout = timeout_secs;
        self
    }

    pub fn with_run_at(mut self, at: DateTime<Utc>) -> Self {
        self.run_at = at;
        self
    }

    /// Delay eligibility by `secs` from now (Rails `set(wait: n.seconds)`).
    pub fn with_wait(mut self, secs: i64) -> Self {
        self.run_at = Utc::now() + chrono::Duration::seconds(secs);
        self
    }

    /// Route the job to a different queue (Rails `set(queue:)`).
    pub fn with_queue(mut self, queue: impl Into<String>) -> Self {
        self.queue = queue.into();
        self
    }

    /// Whether the job is eligible to be reserved at `now`.
    pub fn is_ready(&self, now: DateTime<Utc>) -> bool {
        self.run_at <= now
    }
}

/// A job leased from the queue. It stays invisible to other workers until the
/// lease expires or it is `ack`/`nack`ed.
#[derive(Clone, Debug)]
pub struct Reserved {
    pub job: JobPayload,
    pub lease_until: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait JobQueue: Send + Sync {
    /// Enqueue for immediate eligibility.
    async fn enqueue(&self, job: JobPayload) -> Result<JobId>;

    /// Enqueue, eligible no earlier than `at`.
    async fn enqueue_at(&self, job: JobPayload, at: DateTime<Utc>) -> Result<JobId>;

    /// Atomically lease the next eligible job across `queues` (priority order),
    /// honoring `run_at`. `wait` is an upper bound for how long the backend may
    /// block waiting for work before returning `None`.
    async fn reserve(&self, queues: &[&str], wait: Duration) -> Result<Option<Reserved>>;

    /// Job succeeded — remove it.
    async fn ack(&self, id: &str) -> Result<()>;

    /// Job failed but has retries left — re-enqueue, eligible at `retry_at`
    /// (None = immediately).
    async fn nack(&self, id: &str, retry_at: Option<DateTime<Utc>>, error: &str) -> Result<()>;

    /// Return leased-but-expired jobs to their queue. Returns the count reclaimed.
    async fn reclaim_expired(&self, queues: &[&str]) -> Result<u64>;

    /// Move an exhausted job to the dead letter store.
    async fn dead_letter(&self, id: &str, reason: &str) -> Result<()>;

    /// Inspect the dead letter store for a queue.
    async fn dead_jobs(&self, queue: &str) -> Result<Vec<JobPayload>>;

    /// Discard every dead-lettered job for a queue (Rails `jobs:discard`).
    /// Returns the number of jobs removed.
    async fn discard_dead(&self, queue: &str) -> Result<u64>;
}
