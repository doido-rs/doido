//! Database-backed queue (sea-orm). Enabled with the `jobs-db` feature.
//!
//! Jobs live in a single `doido_jobs` table. `reserve` claims a row atomically via
//! a guarded `UPDATE … WHERE status = 'pending'`, which works on both SQLite and
//! Postgres without `SKIP LOCKED` (the guard ensures only one worker wins a row).

use crate::queue::{JobId, JobPayload, JobQueue, JobStatus, Reserved};
use chrono::{DateTime, Utc};
use doido_core::{anyhow::anyhow, Result};
use doido_model::sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, Value,
};
use std::time::Duration;

const STATUS_PENDING: &str = "pending";
const STATUS_RUNNING: &str = "running";
const STATUS_DEAD: &str = "dead";

/// Default visibility timeout for reserved jobs.
const VISIBILITY_TIMEOUT: Duration = Duration::from_secs(300);

pub struct DbQueue {
    conn: DatabaseConnection,
    visibility_timeout: Duration,
}

fn millis(dt: DateTime<Utc>) -> i64 {
    dt.timestamp_millis()
}

impl DbQueue {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self {
            conn,
            visibility_timeout: VISIBILITY_TIMEOUT,
        }
    }

    pub fn with_visibility_timeout(mut self, timeout: Duration) -> Self {
        self.visibility_timeout = timeout;
        self
    }

    fn backend(&self) -> DatabaseBackend {
        self.conn.get_database_backend()
    }

    fn stmt(&self, sql: &str, values: Vec<Value>) -> Statement {
        Statement::from_sql_and_values(self.backend(), sql, values)
    }

    /// Create the `doido_jobs` table if it does not exist.
    pub async fn migrate(&self) -> Result<()> {
        let sql = "CREATE TABLE IF NOT EXISTS doido_jobs (
            id TEXT PRIMARY KEY,
            queue TEXT NOT NULL,
            status TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 0,
            run_at INTEGER NOT NULL,
            locked_at INTEGER,
            data TEXT NOT NULL
        )";
        self.conn
            .execute_raw(self.stmt(sql, vec![]))
            .await
            .map_err(|e| anyhow!("doido_jobs migrate failed: {e}"))?;
        let idx = "CREATE INDEX IF NOT EXISTS idx_doido_jobs_reserve
            ON doido_jobs (queue, status, run_at)";
        self.conn
            .execute_raw(self.stmt(idx, vec![]))
            .await
            .map_err(|e| anyhow!("doido_jobs index failed: {e}"))?;
        Ok(())
    }

    async fn insert(&self, job: &JobPayload) -> Result<JobId> {
        let data = serde_json::to_string(job)?;
        let sql = "INSERT INTO doido_jobs (id, queue, status, priority, run_at, locked_at, data)
                   VALUES ($1, $2, $3, $4, $5, NULL, $6)";
        self.conn
            .execute_raw(self.stmt(
                sql,
                vec![
                    job.id.clone().into(),
                    job.queue.clone().into(),
                    STATUS_PENDING.into(),
                    (job.priority as i64).into(),
                    millis(job.run_at).into(),
                    data.into(),
                ],
            ))
            .await
            .map_err(|e| anyhow!("enqueue failed: {e}"))?;
        Ok(job.id.clone())
    }

    async fn load(&self, id: &str) -> Result<Option<JobPayload>> {
        let sql = "SELECT data FROM doido_jobs WHERE id = $1";
        let row = self
            .conn
            .query_one_raw(self.stmt(sql, vec![id.into()]))
            .await
            .map_err(|e| anyhow!("load failed: {e}"))?;
        match row {
            Some(r) => {
                let data: String = r.try_get("", "data").map_err(|e| anyhow!("{e}"))?;
                Ok(Some(serde_json::from_str(&data)?))
            }
            None => Ok(None),
        }
    }

    /// Try to claim one ready job. Returns Ok(None) if nothing is ready right now.
    async fn try_claim(&self, queues: &[&str], now: DateTime<Utc>) -> Result<Option<Reserved>> {
        if queues.is_empty() {
            return Ok(None);
        }
        // Build an IN clause with positional placeholders.
        let placeholders: Vec<String> = (0..queues.len()).map(|i| format!("${}", i + 2)).collect();
        let select = format!(
            "SELECT id, data FROM doido_jobs
             WHERE status = $1 AND run_at <= ${n} AND queue IN ({ph})
             ORDER BY priority DESC, run_at ASC
             LIMIT 1",
            n = queues.len() + 2,
            ph = placeholders.join(", "),
        );
        let mut values: Vec<Value> = vec![STATUS_PENDING.into()];
        for q in queues {
            values.push((*q).into());
        }
        values.push(millis(now).into());

        let row = self
            .conn
            .query_one_raw(self.stmt(&select, values))
            .await
            .map_err(|e| anyhow!("reserve select failed: {e}"))?;

        let Some(row) = row else {
            return Ok(None);
        };
        let id: String = row.try_get("", "id").map_err(|e| anyhow!("{e}"))?;
        let data: String = row.try_get("", "data").map_err(|e| anyhow!("{e}"))?;
        let mut job: JobPayload = serde_json::from_str(&data)?;

        // Claim it atomically; the status guard means only one worker wins.
        job.attempts += 1;
        job.status = JobStatus::Running;
        let new_data = serde_json::to_string(&job)?;
        let lease_until = now
            + chrono::Duration::from_std(self.visibility_timeout)
                .unwrap_or_else(|_| chrono::Duration::seconds(300));
        let update = "UPDATE doido_jobs SET status = $1, locked_at = $2, data = $3
                      WHERE id = $4 AND status = $5";
        let res = self
            .conn
            .execute_raw(self.stmt(
                update,
                vec![
                    STATUS_RUNNING.into(),
                    millis(now).into(),
                    new_data.into(),
                    id.clone().into(),
                    STATUS_PENDING.into(),
                ],
            ))
            .await
            .map_err(|e| anyhow!("reserve claim failed: {e}"))?;

        if res.rows_affected() == 1 {
            Ok(Some(Reserved { job, lease_until }))
        } else {
            // Lost the race; let the caller retry.
            Ok(None)
        }
    }
}

#[async_trait::async_trait]
impl JobQueue for DbQueue {
    async fn enqueue(&self, mut job: JobPayload) -> Result<JobId> {
        job.status = JobStatus::Pending;
        self.insert(&job).await
    }

    async fn enqueue_at(&self, mut job: JobPayload, at: DateTime<Utc>) -> Result<JobId> {
        job.status = JobStatus::Pending;
        job.run_at = at;
        self.insert(&job).await
    }

    async fn reserve(&self, queues: &[&str], wait: Duration) -> Result<Option<Reserved>> {
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            if let Some(reserved) = self.try_claim(queues, Utc::now()).await? {
                return Ok(Some(reserved));
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            tokio::time::sleep(remaining.min(Duration::from_millis(50))).await;
        }
    }

    async fn ack(&self, id: &str) -> Result<()> {
        self.conn
            .execute_raw(self.stmt("DELETE FROM doido_jobs WHERE id = $1", vec![id.into()]))
            .await
            .map_err(|e| anyhow!("ack failed: {e}"))?;
        Ok(())
    }

    async fn nack(&self, id: &str, retry_at: Option<DateTime<Utc>>, error: &str) -> Result<()> {
        let Some(mut job) = self.load(id).await? else {
            return Ok(());
        };
        let run_at = retry_at.unwrap_or_else(Utc::now);
        job.status = JobStatus::Pending;
        job.error = Some(error.to_string());
        job.run_at = run_at;
        let data = serde_json::to_string(&job)?;
        let sql = "UPDATE doido_jobs SET status = $1, run_at = $2, locked_at = NULL, data = $3
                   WHERE id = $4";
        self.conn
            .execute_raw(self.stmt(
                sql,
                vec![
                    STATUS_PENDING.into(),
                    millis(run_at).into(),
                    data.into(),
                    id.into(),
                ],
            ))
            .await
            .map_err(|e| anyhow!("nack failed: {e}"))?;
        Ok(())
    }

    async fn reclaim_expired(&self, queues: &[&str]) -> Result<u64> {
        if queues.is_empty() {
            return Ok(0);
        }
        let cutoff = millis(Utc::now()) - self.visibility_timeout.as_millis() as i64;
        // Values: $1 pending, $2 running, $3 cutoff, $4.. queue names.
        let placeholders: Vec<String> = (0..queues.len()).map(|i| format!("${}", i + 4)).collect();
        let sql = format!(
            "UPDATE doido_jobs SET status = $1, locked_at = NULL
             WHERE status = $2 AND locked_at <= $3 AND queue IN ({ph})",
            ph = placeholders.join(", "),
        );
        let mut values: Vec<Value> = vec![STATUS_PENDING.into(), STATUS_RUNNING.into()];
        values.push(cutoff.into());
        for q in queues {
            values.push((*q).into());
        }
        let res = self
            .conn
            .execute_raw(self.stmt(&sql, values))
            .await
            .map_err(|e| anyhow!("reclaim failed: {e}"))?;
        Ok(res.rows_affected())
    }

    async fn dead_letter(&self, id: &str, reason: &str) -> Result<()> {
        let Some(mut job) = self.load(id).await? else {
            return Ok(());
        };
        job.status = JobStatus::Dead;
        job.error = Some(reason.to_string());
        let data = serde_json::to_string(&job)?;
        let sql = "UPDATE doido_jobs SET status = $1, locked_at = NULL, data = $2 WHERE id = $3";
        self.conn
            .execute_raw(self.stmt(
                sql,
                vec![STATUS_DEAD.into(), data.into(), id.into()],
            ))
            .await
            .map_err(|e| anyhow!("dead_letter failed: {e}"))?;
        Ok(())
    }

    async fn dead_jobs(&self, queue: &str) -> Result<Vec<JobPayload>> {
        let sql = "SELECT data FROM doido_jobs WHERE status = $1 AND queue = $2
                   ORDER BY run_at ASC";
        let rows = self
            .conn
            .query_all_raw(self.stmt(sql, vec![STATUS_DEAD.into(), queue.into()]))
            .await
            .map_err(|e| anyhow!("dead_jobs failed: {e}"))?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let data: String = r.try_get("", "data").map_err(|e| anyhow!("{e}"))?;
            out.push(serde_json::from_str(&data)?);
        }
        Ok(out)
    }
}
