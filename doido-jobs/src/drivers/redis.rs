//! Redis-backed queue. Enabled with the `jobs-redis` feature.
//!
//! Layout (keys are namespaced, default `doido:jobs`):
//! - `{ns}:job:{id}`        — JSON-encoded `JobPayload`.
//! - `{ns}:ready:{queue}`   — ZSET of job ids, scored by `run_at_millis - priority`
//!   so that due, higher-priority jobs sort first and scheduled jobs stay in the future.
//! - `{ns}:processing`      — ZSET of leased job ids scored by `lease_until_millis`
//!   (the reliable-queue "in-flight" set; reclaimed when the score is in the past).
//! - `{ns}:dead:{queue}`    — LIST of dead-lettered job ids.
//!
//! `reserve` is atomic via a small Lua script: it pops the lowest-scored due id from
//! the ready set and records it in the processing set in one round trip.

use crate::inspect::{
    count_stored, matches_bucket, paginate_sorted, JobBucket, JobListPage, JobPageAnchor,
    QueueCounts,
};
use crate::queue::{JobId, JobPayload, JobQueue, JobStatus, Reserved};
use chrono::{DateTime, Utc};
use doido_core::{anyhow::anyhow, Result};
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Script};
use std::time::Duration;

const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(300);

pub struct RedisQueue {
    conn: MultiplexedConnection,
    namespace: String,
    visibility_timeout: Duration,
}

fn millis(dt: DateTime<Utc>) -> i64 {
    dt.timestamp_millis()
}

/// Atomic reserve: pop the lowest-scored id that is due (score <= now) from the
/// ready set and add it to the processing set with its lease deadline.
const RESERVE_SCRIPT: &str = r#"
local ids = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, 1)
if #ids == 0 then return false end
local id = ids[1]
redis.call('ZREM', KEYS[1], id)
redis.call('ZADD', KEYS[2], ARGV[2], id)
return id
"#;

impl RedisQueue {
    pub async fn connect(url: &str, namespace: impl Into<String>) -> Result<Self> {
        let client = redis::Client::open(url).map_err(|e| anyhow!("redis open failed: {e}"))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| anyhow!("redis connect failed: {e}"))?;
        Ok(Self {
            conn,
            namespace: namespace.into(),
            visibility_timeout: VISIBILITY_TIMEOUT,
        })
    }

    pub fn with_visibility_timeout(mut self, timeout: Duration) -> Self {
        self.visibility_timeout = timeout;
        self
    }

    fn job_key(&self, id: &str) -> String {
        format!("{}:job:{}", self.namespace, id)
    }

    fn ready_key(&self, queue: &str) -> String {
        format!("{}:ready:{}", self.namespace, queue)
    }

    fn processing_key(&self) -> String {
        format!("{}:processing", self.namespace)
    }

    fn dead_key(&self, queue: &str) -> String {
        format!("{}:dead:{}", self.namespace, queue)
    }

    fn score(&self, job: &JobPayload) -> f64 {
        // Lower score sorts first: earlier run_at, then higher priority.
        millis(job.run_at) as f64 - job.priority as f64
    }

    async fn store(&self, job: &JobPayload) -> Result<()> {
        let mut conn = self.conn.clone();
        let data = serde_json::to_string(job)?;
        let _: () = conn
            .set(self.job_key(&job.id), data)
            .await
            .map_err(|e| anyhow!("store job failed: {e}"))?;
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<JobPayload>> {
        let id = id.to_string();
        let mut jobs = self.load_many(std::slice::from_ref(&id)).await?;
        Ok(jobs.pop())
    }

    async fn load_many(&self, ids: &[String]) -> Result<Vec<JobPayload>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        const BATCH: usize = 100;
        let mut out = Vec::with_capacity(ids.len());
        let mut conn = self.conn.clone();
        for chunk in ids.chunks(BATCH) {
            let keys: Vec<String> = chunk.iter().map(|id| self.job_key(id)).collect();
            let values: Vec<Option<String>> = conn
                .mget(keys)
                .await
                .map_err(|e| anyhow!("mget jobs failed: {e}"))?;
            for (id, data) in chunk.iter().zip(values) {
                let Some(data) = data else {
                    continue;
                };
                let mut job: JobPayload = serde_json::from_str(&data)?;
                if job.id.is_empty() {
                    job.id = id.clone();
                }
                out.push(job);
            }
        }
        Ok(out)
    }

    fn strip_light(jobs: &mut [JobPayload], light: bool) {
        if !light {
            return;
        }
        for job in jobs.iter_mut() {
            job.payload = serde_json::json!({});
        }
    }

    async fn collect_ready_bucket_jobs(
        &self,
        queue: &str,
        bucket: JobBucket,
        now: DateTime<Utc>,
        conn: &mut MultiplexedConnection,
        matches: &mut Vec<JobPayload>,
    ) -> Result<()> {
        const CHUNK: isize = 200;
        let key = self.ready_key(queue);
        let mut start: isize = 0;
        loop {
            let end = start + CHUNK - 1;
            let ids: Vec<String> = conn
                .zrange(&key, start, end)
                .await
                .map_err(|e| anyhow!("list ready chunk: {e}"))?;
            if ids.is_empty() {
                break;
            }
            start += ids.len() as isize;
            for job in self.load_many(&ids).await? {
                let in_bucket = match bucket {
                    JobBucket::InQueue => {
                        matches_bucket(&job, JobBucket::PendingReady, now)
                            || matches_bucket(&job, JobBucket::FailedReady, now)
                    }
                    _ => matches_bucket(&job, bucket, now),
                };
                if in_bucket {
                    matches.push(job);
                }
            }
            if ids.len() < CHUNK as usize {
                break;
            }
        }
        Ok(())
    }

    async fn put_ready(&self, job: &JobPayload) -> Result<()> {
        self.store(job).await?;
        let mut conn = self.conn.clone();
        let _: () = conn
            .zadd(self.ready_key(&job.queue), job.id.clone(), self.score(job))
            .await
            .map_err(|e| anyhow!("enqueue zadd failed: {e}"))?;
        Ok(())
    }

    async fn try_reserve_queue(&self, queue: &str, now: DateTime<Utc>) -> Result<Option<Reserved>> {
        let mut conn = self.conn.clone();
        let lease_until = now
            + chrono::Duration::from_std(self.visibility_timeout)
                .unwrap_or_else(|_| chrono::Duration::seconds(300));
        let id: Option<String> = Script::new(RESERVE_SCRIPT)
            .key(self.ready_key(queue))
            .key(self.processing_key())
            .arg(millis(now))
            .arg(millis(lease_until))
            .invoke_async(&mut conn)
            .await
            .map_err(|e| anyhow!("reserve script failed: {e}"))?;

        let Some(id) = id else {
            return Ok(None);
        };
        let Some(mut job) = self.load(&id).await? else {
            // Orphaned id with no payload; drop it from processing.
            let _: () = conn
                .zrem(self.processing_key(), &id)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            return Ok(None);
        };
        job.attempts += 1;
        job.status = JobStatus::Running;
        self.store(&job).await?;
        Ok(Some(Reserved { job, lease_until }))
    }
}

#[async_trait::async_trait]
impl JobQueue for RedisQueue {
    async fn enqueue(&self, mut job: JobPayload) -> Result<JobId> {
        job.status = JobStatus::Pending;
        let id = job.id.clone();
        self.put_ready(&job).await?;
        Ok(id)
    }

    async fn enqueue_at(&self, mut job: JobPayload, at: DateTime<Utc>) -> Result<JobId> {
        job.status = JobStatus::Pending;
        job.run_at = at;
        let id = job.id.clone();
        self.put_ready(&job).await?;
        Ok(id)
    }

    async fn reserve(&self, queues: &[&str], wait: Duration) -> Result<Option<Reserved>> {
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            let now = Utc::now();
            for queue in queues {
                if let Some(reserved) = self.try_reserve_queue(queue, now).await? {
                    return Ok(Some(reserved));
                }
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            tokio::time::sleep(remaining.min(Duration::from_millis(50))).await;
        }
    }

    async fn ack(&self, id: &str) -> Result<()> {
        let mut conn = self.conn.clone();
        let _: () = conn
            .zrem(self.processing_key(), id)
            .await
            .map_err(|e| anyhow!("ack zrem failed: {e}"))?;
        let _: () = conn
            .del(self.job_key(id))
            .await
            .map_err(|e| anyhow!("ack del failed: {e}"))?;
        Ok(())
    }

    async fn nack(&self, id: &str, retry_at: Option<DateTime<Utc>>, error: &str) -> Result<()> {
        let Some(mut job) = self.load(id).await? else {
            return Ok(());
        };
        let mut conn = self.conn.clone();
        let _: () = conn
            .zrem(self.processing_key(), id)
            .await
            .map_err(|e| anyhow!("nack zrem failed: {e}"))?;
        job.status = JobStatus::Failed;
        job.error = Some(error.to_string());
        job.run_at = retry_at.unwrap_or_else(Utc::now);
        self.put_ready(&job).await?;
        Ok(())
    }

    async fn reclaim_expired(&self, queues: &[&str]) -> Result<u64> {
        let mut conn = self.conn.clone();
        let now = millis(Utc::now());
        let expired: Vec<String> = conn
            .zrangebyscore_limit(self.processing_key(), "-inf", now, 0, 1000)
            .await
            .map_err(|e| anyhow!("reclaim range failed: {e}"))?;
        let mut count = 0u64;
        for id in expired {
            let Some(job) = self.load(&id).await? else {
                let _: () = conn
                    .zrem(self.processing_key(), &id)
                    .await
                    .map_err(|e| anyhow!("{e}"))?;
                continue;
            };
            if !queues.contains(&job.queue.as_str()) {
                continue;
            }
            let _: () = conn
                .zrem(self.processing_key(), &id)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            self.put_ready(&job).await?;
            count += 1;
        }
        Ok(count)
    }

    async fn dead_letter(&self, id: &str, reason: &str) -> Result<()> {
        let Some(mut job) = self.load(id).await? else {
            return Ok(());
        };
        let mut conn = self.conn.clone();
        let _: () = conn
            .zrem(self.processing_key(), id)
            .await
            .map_err(|e| anyhow!("dead_letter zrem failed: {e}"))?;
        job.status = JobStatus::Dead;
        job.error = Some(reason.to_string());
        self.store(&job).await?;
        let _: () = conn
            .rpush(self.dead_key(&job.queue), id)
            .await
            .map_err(|e| anyhow!("dead_letter rpush failed: {e}"))?;
        Ok(())
    }

    async fn dead_jobs(&self, queue: &str) -> Result<Vec<JobPayload>> {
        let mut conn = self.conn.clone();
        let ids: Vec<String> = conn
            .lrange(self.dead_key(queue), 0, -1)
            .await
            .map_err(|e| anyhow!("dead_jobs lrange failed: {e}"))?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(job) = self.load(&id).await? {
                out.push(job);
            }
        }
        Ok(out)
    }

    async fn discard_dead(&self, queue: &str) -> Result<u64> {
        let mut conn = self.conn.clone();
        let ids: Vec<String> = conn
            .lrange(self.dead_key(queue), 0, -1)
            .await
            .map_err(|e| anyhow!("discard_dead lrange failed: {e}"))?;
        let count = ids.len() as u64;
        for id in &ids {
            let _: () = conn
                .del(self.job_key(id))
                .await
                .map_err(|e| anyhow!("discard_dead del failed: {e}"))?;
        }
        let _: () = conn
            .del(self.dead_key(queue))
            .await
            .map_err(|e| anyhow!("discard_dead del list failed: {e}"))?;
        Ok(count)
    }

    async fn queue_counts(&self, queues: &[&str]) -> Result<Vec<(String, QueueCounts)>> {
        let now = Utc::now();
        let mut conn = self.conn.clone();
        let queue_set: std::collections::HashSet<&str> = queues.iter().copied().collect();
        let mut running_by_queue: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        let processing_ids: Vec<String> = conn
            .zrange(self.processing_key(), 0, -1)
            .await
            .map_err(|e| anyhow!("queue_counts processing: {e}"))?;
        for job in self.load_many(&processing_ids).await? {
            if queue_set.contains(job.queue.as_str()) {
                *running_by_queue.entry(job.queue.clone()).or_default() += 1;
            }
        }

        let mut out = Vec::with_capacity(queues.len());
        for name in queues {
            let mut counts = QueueCounts {
                running: running_by_queue.get(*name).copied().unwrap_or(0),
                ..QueueCounts::default()
            };
            let ids: Vec<String> = conn
                .zrange(self.ready_key(name), 0, -1)
                .await
                .map_err(|e| anyhow!("queue_counts zrange: {e}"))?;
            for job in self.load_many(&ids).await? {
                count_stored(&mut counts, &job, now);
            }
            let dead_len: usize = conn
                .llen(self.dead_key(name))
                .await
                .map_err(|e| anyhow!("queue_counts dead llen: {e}"))?;
            counts.dead = dead_len as u32;
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
        let mut matches = Vec::new();
        let mut conn = self.conn.clone();
        if bucket == JobBucket::Running {
            let ids: Vec<String> = conn
                .zrange(self.processing_key(), 0, -1)
                .await
                .map_err(|e| anyhow!("list running: {e}"))?;
            for job in self.load_many(&ids).await? {
                if job.queue == queue {
                    matches.push(job);
                }
            }
        } else if bucket == JobBucket::Dead {
            let ids: Vec<String> = conn
                .lrange(self.dead_key(queue), 0, -1)
                .await
                .map_err(|e| anyhow!("list dead: {e}"))?;
            matches.extend(self.load_many(&ids).await?);
        } else {
            self.collect_ready_bucket_jobs(queue, bucket, now, &mut conn, &mut matches)
                .await?;
        }
        Self::strip_light(&mut matches, light);
        Ok(paginate_sorted(matches, page, limit))
    }

    async fn retry_dead_job(&self, queue: &str, job_id: &str) -> Result<()> {
        let jobs = self.dead_jobs(queue).await?;
        let Some(job) = jobs.into_iter().find(|j| j.id == job_id) else {
            return Ok(());
        };
        if self.discard_dead_job(queue, job_id).await? {
            self.enqueue(job.requeue_fresh()).await?;
        }
        Ok(())
    }

    async fn discard_dead_job(&self, queue: &str, job_id: &str) -> Result<bool> {
        let mut conn = self.conn.clone();
        let removed: i32 = conn
            .lrem(self.dead_key(queue), 1, job_id)
            .await
            .map_err(|e| anyhow!("discard_dead_job lrem: {e}"))?;
        if removed > 0 {
            let _: () = conn
                .del(self.job_key(job_id))
                .await
                .map_err(|e| anyhow!("discard_dead_job del: {e}"))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn cancel_job(&self, queue: &str, job_id: &str) -> Result<bool> {
        let Some(job) = self.load(job_id).await? else {
            return Ok(false);
        };
        if job.queue != queue || job.status == JobStatus::Dead {
            return Ok(false);
        }
        let mut conn = self.conn.clone();
        let _: () = conn
            .zrem(self.ready_key(queue), job_id)
            .await
            .map_err(|e| anyhow!("cancel_job zrem ready: {e}"))?;
        let _: () = conn
            .zrem(self.processing_key(), job_id)
            .await
            .map_err(|e| anyhow!("cancel_job zrem processing: {e}"))?;
        let _: () = conn
            .lrem(self.dead_key(queue), 0, job_id)
            .await
            .map_err(|e| anyhow!("cancel_job lrem dead: {e}"))?;
        let deleted: i32 = conn
            .del(self.job_key(job_id))
            .await
            .map_err(|e| anyhow!("cancel_job del: {e}"))?;
        Ok(deleted > 0)
    }
}
