use doido_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Dead,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobPayload {
    pub id: String,
    pub queue: String,
    pub payload: Value,
    pub attempts: u32,
    pub max_retries: u32,
    pub status: JobStatus,
    pub error: Option<String>,
}

impl JobPayload {
    pub fn new(queue: impl Into<String>, payload: Value, max_retries: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            queue: queue.into(),
            payload,
            attempts: 0,
            max_retries,
            status: JobStatus::Pending,
            error: None,
        }
    }
}

#[async_trait::async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue(&self, payload: JobPayload) -> Result<()>;
    async fn dequeue(&self, queue: &str) -> Result<Option<JobPayload>>;
    async fn ack(&self, id: &str) -> Result<()>;
    async fn nack(&self, id: &str, error: &str) -> Result<()>;
    async fn dead_jobs(&self, queue: &str) -> Result<Vec<JobPayload>>;
}
