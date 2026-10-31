use crate::inspect::{
    count_stored, job_sort_cmp, matches_bucket, paginate_sorted, JobBucket, JobListPage,
    JobPageAnchor, JobSortKey, QueueCounts,
};
use crate::queue::{JobId, JobPayload, JobQueue, JobStatus, Reserved};
use chrono::{DateTime, Utc};
use doido_core::Result;
use serde_json::json;
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

/// Introspection counts for a queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueueStats {
    pub pending: usize,
    pub dead: usize,
}

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

    /// Introspection: pending and dead-letter job counts for `queue` (Rails
    /// `bin/jobs` / job dashboards).
    pub async fn stats(&self, queue: &str) -> QueueStats {
        let pending = self
            .queues
            .lock()
            .await
            .get(queue)
            .map(Vec::len)
            .unwrap_or(0);
        let dead = self.dead.lock().await.get(queue).map(Vec::len).unwrap_or(0);
        QueueStats { pending, dead }
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
            let _ = tokio::time::timeout(
                remaining.min(Duration::from_millis(50)),
                self.notify.notified(),
            )
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
            job.status = JobStatus::Failed;
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

    async fn discard_dead(&self, queue: &str) -> Result<u64> {
        let removed = self
            .dead
            .lock()
            .await
            .remove(queue)
            .map(|jobs| jobs.len() as u64)
            .unwrap_or(0);
        Ok(removed)
    }

    async fn queue_counts(&self, queues: &[&str]) -> Result<Vec<(String, QueueCounts)>> {
        let now = Utc::now();
        let mut out = Vec::with_capacity(queues.len());
        let qs = self.queues.lock().await;
        let running = self.running.lock().await;
        let dead = self.dead.lock().await;
        for name in queues {
            let mut counts = QueueCounts::default();
            if let Some(list) = qs.get(*name) {
                for job in list {
                    count_stored(&mut counts, job, now);
                }
            }
            for leased in running.values() {
                if leased.job.queue == *name {
                    counts.running += 1;
                }
            }
            counts.dead = dead.get(*name).map(|v| v.len() as u32).unwrap_or(0);
            out.push(((*name).to_string(), counts));
        }
        Ok(out)
    }

    async fn list_jobs(
        &self,
        queue: &str,
        bucket: JobBucket,
        limit: u32,
        page: Option<JobPageAnchor>,
        light: bool,
    ) -> Result<JobListPage> {
        let now = Utc::now();
        let sort_keys = self.collect_bucket_sort_keys(queue, bucket, now).await;
        let stubs: Vec<JobPayload> = sort_keys.iter().map(job_stub_from_key).collect();
        let page = paginate_sorted(stubs, page, limit);
        if page.jobs.is_empty() {
            return Ok(page);
        }
        let want: std::collections::HashSet<&str> =
            page.jobs.iter().map(|j| j.id.as_str()).collect();
        let mut materialized = self
            .materialize_jobs(queue, bucket, now, &want, light)
            .await;
        materialized.sort_by(job_sort_cmp);
        Ok(JobListPage {
            jobs: materialized,
            next_page_token: page.next_page_token,
            prev_page_token: page.prev_page_token,
        })
    }

    async fn retry_dead_job(&self, queue: &str, job_id: &str) -> Result<()> {
        let job = {
            let mut dead = self.dead.lock().await;
            let Some(list) = dead.get_mut(queue) else {
                return Ok(());
            };
            let pos = list.iter().position(|j| j.id == job_id);
            let Some(idx) = pos else {
                return Ok(());
            };
            list.remove(idx)
        };
        self.enqueue(job.requeue_fresh()).await?;
        Ok(())
    }

    async fn discard_dead_job(&self, queue: &str, job_id: &str) -> Result<bool> {
        let mut dead = self.dead.lock().await;
        let Some(list) = dead.get_mut(queue) else {
            return Ok(false);
        };
        let pos = list.iter().position(|j| j.id == job_id);
        let Some(idx) = pos else {
            return Ok(false);
        };
        list.remove(idx);
        Ok(true)
    }

    async fn cancel_job(&self, queue: &str, job_id: &str) -> Result<bool> {
        {
            let mut qs = self.queues.lock().await;
            if let Some(list) = qs.get_mut(queue) {
                if let Some(idx) = list.iter().position(|j| j.id == job_id) {
                    list.remove(idx);
                    self.notify.notify_waiters();
                    return Ok(true);
                }
            }
        }
        let removed = self.running.lock().await.remove(job_id).is_some();
        Ok(removed)
    }
}

impl MemoryQueue {
    async fn collect_bucket_sort_keys(
        &self,
        queue: &str,
        bucket: JobBucket,
        now: DateTime<Utc>,
    ) -> Vec<JobSortKey> {
        let mut keys = Vec::new();
        if bucket == JobBucket::Running {
            let running = self.running.lock().await;
            for leased in running.values() {
                if leased.job.queue == queue {
                    keys.push(JobSortKey::from_job(&leased.job));
                }
            }
        } else if bucket == JobBucket::Dead {
            let dead = self.dead.lock().await;
            if let Some(list) = dead.get(queue) {
                keys.extend(list.iter().map(JobSortKey::from_job));
            }
        } else {
            let qs = self.queues.lock().await;
            if let Some(list) = qs.get(queue) {
                for job in list {
                    let in_bucket = match bucket {
                        JobBucket::InQueue => {
                            matches_bucket(job, JobBucket::PendingReady, now)
                                || matches_bucket(job, JobBucket::FailedReady, now)
                        }
                        _ => matches_bucket(job, bucket, now),
                    };
                    if in_bucket {
                        keys.push(JobSortKey::from_job(job));
                    }
                }
            }
        }
        keys
    }

    async fn materialize_jobs(
        &self,
        queue: &str,
        bucket: JobBucket,
        now: DateTime<Utc>,
        want: &std::collections::HashSet<&str>,
        light: bool,
    ) -> Vec<JobPayload> {
        let mut out = Vec::with_capacity(want.len());
        if bucket == JobBucket::Running {
            let running = self.running.lock().await;
            for leased in running.values() {
                if leased.job.queue == queue && want.contains(leased.job.id.as_str()) {
                    out.push(clone_for_list(&leased.job, light));
                }
            }
        } else if bucket == JobBucket::Dead {
            let dead = self.dead.lock().await;
            if let Some(list) = dead.get(queue) {
                for job in list {
                    if want.contains(job.id.as_str()) {
                        out.push(clone_for_list(job, light));
                    }
                }
            }
        } else {
            let qs = self.queues.lock().await;
            if let Some(list) = qs.get(queue) {
                for job in list {
                    if !want.contains(job.id.as_str()) {
                        continue;
                    }
                    let in_bucket = match bucket {
                        JobBucket::InQueue => {
                            matches_bucket(job, JobBucket::PendingReady, now)
                                || matches_bucket(job, JobBucket::FailedReady, now)
                        }
                        _ => matches_bucket(job, bucket, now),
                    };
                    if in_bucket {
                        out.push(clone_for_list(job, light));
                    }
                }
            }
        }
        out
    }
}

fn job_stub_from_key(key: &JobSortKey) -> JobPayload {
    let mut j = JobPayload::new("default", json!({}), 0);
    j.id = key.id.clone();
    j.priority = key.priority;
    j.run_at = DateTime::from_timestamp_millis(key.run_at_ms).unwrap_or_else(Utc::now);
    j
}

fn clone_for_list(job: &JobPayload, light: bool) -> JobPayload {
    if !light {
        return job.clone();
    }
    let mut j = job.clone();
    j.payload = json!({});
    j
}
