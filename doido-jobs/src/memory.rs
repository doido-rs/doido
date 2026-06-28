use crate::queue::{JobId, JobPayload, JobQueue, JobStatus, Reserved};
use chrono::{DateTime, Utc};
use doido_core::Result;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};

/// A leased job held by a worker, with the instant its lease expires.
struct Leased {
    job: JobPayload,
    lease_until: DateTime<Utc>,
}

/// Default visibility timeout for reserved jobs.
const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(300);

pub struct MemoryQueue {
    /// Pending jobs per queue name (includes scheduled jobs not yet due).
    queues: Mutex<HashMap<String, Vec<JobPayload>>>,
    /// Dead-lettered jobs per queue name.
    dead: Mutex<HashMap<String, Vec<JobPayload>>>,
    /// In-flight leased jobs keyed by job id.
    running: Mutex<HashMap<JobId, Leased>>,
    /// Visibility timeout applied on reserve.
    visibility_timeout: Duration,
    /// Wakes reservers when new work is enqueued.
    notify: Notify,
}

impl MemoryQueue {
    pub fn new() -> Self {
        Self {
            queues: Mutex::new(HashMap::new()),
            dead: Mutex::new(HashMap::new()),
            running: Mutex::new(HashMap::new()),
            visibility_timeout: VISIBILITY_TIMEOUT,
            notify: Notify::new(),
        }
    }

    pub fn with_visibility_timeout(mut self, timeout: Duration) -> Self {
        self.visibility_timeout = timeout;
        self
    }

    async fn push(&self, job: JobPayload) {
        self.queues
            .lock()
            .await
            .entry(job.queue.clone())
            .or_default()
            .push(job);
        self.notify.notify_waiters();
    }

    /// Pop the highest-priority, earliest-eligible ready job across `queues`.
    async fn pop_ready(&self, queues: &[&str], now: DateTime<Utc>) -> Option<JobPayload> {
        let mut qs = self.queues.lock().await;
        // Honor queue priority order: first listed queue wins ties.
        for name in queues {
            if let Some(list) = qs.get_mut(*name) {
                // Pick the best ready candidate: highest priority, then earliest run_at.
                let mut best: Option<usize> = None;
                for (i, job) in list.iter().enumerate() {
                    if !job.is_ready(now) {
                        continue;
                    }
                    match best {
                        None => best = Some(i),
                        Some(b) => {
                            let cur = &list[b];
                            if job.priority > cur.priority
                                || (job.priority == cur.priority && job.run_at < cur.run_at)
                            {
                                best = Some(i);
                            }
                        }
                    }
                }
                if let Some(i) = best {
                    return Some(list.remove(i));
                }
            }
        }
        None
    }
}

impl Default for MemoryQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl JobQueue for MemoryQueue {
    async fn enqueue(&self, mut job: JobPayload) -> Result<JobId> {
        job.status = JobStatus::Pending;
        let id = job.id.clone();
        self.push(job).await;
        Ok(id)
    }

    async fn enqueue_at(&self, mut job: JobPayload, at: DateTime<Utc>) -> Result<JobId> {
        job.status = JobStatus::Pending;
        job.run_at = at;
        let id = job.id.clone();
        self.push(job).await;
        Ok(id)
    }

    async fn reserve(&self, queues: &[&str], wait: Duration) -> Result<Option<Reserved>> {
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            let now = Utc::now();
            if let Some(mut job) = self.pop_ready(queues, now).await {
                job.status = JobStatus::Running;
                job.attempts += 1;
                let lease_until = now
                    + chrono::Duration::from_std(self.visibility_timeout)
                        .unwrap_or_else(|_| chrono::Duration::seconds(300));
                self.running.lock().await.insert(
                    job.id.clone(),
                    Leased {
                        job: job.clone(),
                        lease_until,
                    },
                );
                return Ok(Some(Reserved { job, lease_until }));
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            // Park until notified or the wait window elapses, then re-check (also
            // catches scheduled jobs becoming due).
            let _ = tokio::time::timeout(remaining.min(Duration::from_millis(50)), self.notify.notified())
                .await;
        }
    }

    async fn ack(&self, id: &str) -> Result<()> {
        self.running.lock().await.remove(id);
        Ok(())
    }

    async fn nack(&self, id: &str, retry_at: Option<DateTime<Utc>>, error: &str) -> Result<()> {
        if let Some(Leased { mut job, .. }) = self.running.lock().await.remove(id) {
            job.error = Some(error.to_string());
            job.status = JobStatus::Pending;
            job.run_at = retry_at.unwrap_or_else(Utc::now);
            self.push(job).await;
        }
        Ok(())
    }

    async fn reclaim_expired(&self, queues: &[&str]) -> Result<u64> {
        let now = Utc::now();
        let mut reclaimed = Vec::new();
        {
            let mut running = self.running.lock().await;
            let expired_ids: Vec<JobId> = running
                .iter()
                .filter(|(_, l)| l.lease_until <= now && queues.contains(&l.job.queue.as_str()))
                .map(|(id, _)| id.clone())
                .collect();
            for id in expired_ids {
                if let Some(Leased { mut job, .. }) = running.remove(&id) {
                    job.status = JobStatus::Pending;
                    reclaimed.push(job);
                }
            }
        }
        let count = reclaimed.len() as u64;
        for job in reclaimed {
            self.push(job).await;
        }
        Ok(count)
    }

    async fn dead_letter(&self, id: &str, reason: &str) -> Result<()> {
        if let Some(Leased { mut job, .. }) = self.running.lock().await.remove(id) {
            job.status = JobStatus::Dead;
            job.error = Some(reason.to_string());
            self.dead
                .lock()
                .await
                .entry(job.queue.clone())
                .or_default()
                .push(job);
        }
        Ok(())
    }

    async fn dead_jobs(&self, queue: &str) -> Result<Vec<JobPayload>> {
        Ok(self
            .dead
            .lock()
            .await
            .get(queue)
            .cloned()
            .unwrap_or_default())
    }
}
