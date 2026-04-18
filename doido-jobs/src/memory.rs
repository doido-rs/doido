use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;
use crate::queue::{JobPayload, JobQueue, JobStatus};
use doido_core::Result;

pub struct MemoryQueue {
    queues: Mutex<HashMap<String, VecDeque<JobPayload>>>,
    dead: Mutex<HashMap<String, Vec<JobPayload>>>,
    running: Mutex<HashMap<String, JobPayload>>,
}

impl MemoryQueue {
    pub fn new() -> Self {
        Self {
            queues: Mutex::new(HashMap::new()),
            dead: Mutex::new(HashMap::new()),
            running: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryQueue {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl JobQueue for MemoryQueue {
    async fn enqueue(&self, mut payload: JobPayload) -> Result<()> {
        payload.status = JobStatus::Pending;
        self.queues.lock().await
            .entry(payload.queue.clone())
            .or_default()
            .push_back(payload);
        Ok(())
    }

    async fn dequeue(&self, queue: &str) -> Result<Option<JobPayload>> {
        let mut qs = self.queues.lock().await;
        if let Some(q) = qs.get_mut(queue) {
            if let Some(mut job) = q.pop_front() {
                job.status = JobStatus::Running;
                job.attempts += 1;
                self.running.lock().await.insert(job.id.clone(), job.clone());
                return Ok(Some(job));
            }
        }
        Ok(None)
    }

    async fn ack(&self, id: &str) -> Result<()> {
        if let Some(mut job) = self.running.lock().await.remove(id) {
            job.status = JobStatus::Done;
        }
        Ok(())
    }

    async fn nack(&self, id: &str, error: &str) -> Result<()> {
        if let Some(mut job) = self.running.lock().await.remove(id) {
            job.error = Some(error.to_string());
            if job.attempts >= job.max_retries {
                job.status = JobStatus::Dead;
                self.dead.lock().await
                    .entry(job.queue.clone())
                    .or_default()
                    .push(job);
            } else {
                job.status = JobStatus::Pending;
                self.queues.lock().await
                    .entry(job.queue.clone())
                    .or_default()
                    .push_back(job);
            }
        }
        Ok(())
    }

    async fn dead_jobs(&self, queue: &str) -> Result<Vec<JobPayload>> {
        Ok(self.dead.lock().await.get(queue).cloned().unwrap_or_default())
    }
}
