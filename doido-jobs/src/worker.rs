use std::sync::Arc;
use crate::queue::{JobPayload, JobQueue};
use doido_core::Result;

pub struct Worker {
    queue: Arc<dyn JobQueue>,
    queue_name: String,
}

impl Worker {
    pub fn new(queue: Arc<dyn JobQueue>, queue_name: impl Into<String>) -> Self {
        Self { queue, queue_name: queue_name.into() }
    }

    pub async fn run_once<F, Fut>(&self, performer: F) -> Result<()>
    where
        F: Fn(JobPayload) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        if let Some(job) = self.queue.dequeue(&self.queue_name).await? {
            let id = job.id.clone();
            match performer(job).await {
                Ok(()) => self.queue.ack(&id).await?,
                Err(e) => self.queue.nack(&id, &e.to_string()).await?,
            }
        }
        Ok(())
    }
}
