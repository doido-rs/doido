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

#[cfg(test)]
mod tests {
    use super::MemoryQueue;
    use crate::queue::{JobPayload, JobQueue, JobStatus};
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_enqueue_and_dequeue() {
        let q = Arc::new(MemoryQueue::new());
        let job = JobPayload::new("default", json!({"x": 1}), 3);
        q.enqueue(job).await.unwrap();
        let dequeued = q.dequeue("default").await.unwrap();
        assert!(dequeued.is_some());
        let j = dequeued.unwrap();
        assert_eq!(j.status, JobStatus::Running);
        assert_eq!(j.attempts, 1);
    }

    #[tokio::test]
    async fn test_ack_removes_from_running() {
        let q = MemoryQueue::new();
        let job = JobPayload::new("default", json!({}), 3);
        q.enqueue(job).await.unwrap();
        let j = q.dequeue("default").await.unwrap().unwrap();
        q.ack(&j.id).await.unwrap();
        // dequeue again — should be empty
        assert!(q.dequeue("default").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_nack_re_enqueues_if_retries_remain() {
        let q = MemoryQueue::new();
        let job = JobPayload::new("default", json!({}), 3);
        q.enqueue(job).await.unwrap();
        let j = q.dequeue("default").await.unwrap().unwrap();
        q.nack(&j.id, "transient error").await.unwrap();
        // should be back in queue
        let j2 = q.dequeue("default").await.unwrap();
        assert!(j2.is_some());
        assert_eq!(j2.unwrap().attempts, 2);
    }

    #[tokio::test]
    async fn test_nack_moves_to_dead_after_max_retries() {
        let q = MemoryQueue::new();
        let job = JobPayload::new("default", json!({}), 1);
        q.enqueue(job).await.unwrap();
        let j = q.dequeue("default").await.unwrap().unwrap();
        // attempts is now 1, max_retries is 1 → should go dead
        q.nack(&j.id, "fatal").await.unwrap();
        let dead = q.dead_jobs("default").await.unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].status, JobStatus::Dead);
    }
}
