//! Database-backed queue (sea-orm). Enabled with the `jobs-db` feature.
//!
//! Jobs live in a single `doido_jobs` table. `reserve` claims a row atomically via
//! a guarded update (`status = pending`), which works on both SQLite and Postgres
//! without `SKIP LOCKED` (the guard ensures only one worker wins a row).

use crate::entity;
use crate::queue::{JobId, JobPayload, JobQueue, JobStatus, Reserved};
use chrono::{DateTime, Utc};
use doido_core::{anyhow::anyhow, Result};
use doido_model::sea_orm::sea_query::{Alias, ColumnDef, Expr, Index, Table as SqTable};
use doido_model::sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set,
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

fn job_from_data(data: &str) -> Result<JobPayload> {
    Ok(serde_json::from_str(data)?)
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

    /// Create the `doido_jobs` table if it does not exist.
    pub async fn migrate(&self) -> Result<()> {
        let mut stmt = SqTable::create();
        stmt.table(Alias::new("doido_jobs")).if_not_exists();

        let mut id = ColumnDef::new(Alias::new("id"));
        id.string().not_null().primary_key();
        stmt.col(id);

        let mut queue = ColumnDef::new(Alias::new("queue"));
        queue.string().not_null();
        stmt.col(queue);

        let mut status = ColumnDef::new(Alias::new("status"));
        status.string().not_null();
        stmt.col(status);

        let mut priority = ColumnDef::new(Alias::new("priority"));
        priority
            .big_integer()
            .not_null()
            .default(Expr::value(0i64));
        stmt.col(priority);

        let mut run_at = ColumnDef::new(Alias::new("run_at"));
        run_at.big_integer().not_null();
        stmt.col(run_at);

        let mut locked_at = ColumnDef::new(Alias::new("locked_at"));
        locked_at.big_integer();
        stmt.col(locked_at);

        let mut data = ColumnDef::new(Alias::new("data"));
        data.string().not_null();
        stmt.col(data);

        self.conn
            .execute(&stmt)
            .await
            .map_err(|e| anyhow!("doido_jobs migrate failed: {e}"))?;

        let mut idx = Index::create();
        idx.name("idx_doido_jobs_reserve")
            .table(Alias::new("doido_jobs"))
            .if_not_exists()
            .col(Alias::new("queue"))
            .col(Alias::new("status"))
            .col(Alias::new("run_at"));
        self.conn
            .execute(&idx)
            .await
            .map_err(|e| anyhow!("doido_jobs index failed: {e}"))?;

        Ok(())
    }

    async fn insert(&self, job: &JobPayload) -> Result<JobId> {
        let data = serde_json::to_string(job)?;
        entity::ActiveModel {
            id: Set(job.id.clone()),
            queue: Set(job.queue.clone()),
            status: Set(STATUS_PENDING.to_string()),
            priority: Set(job.priority as i64),
            run_at: Set(millis(job.run_at)),
            locked_at: Set(None),
            data: Set(data),
        }
        .insert(&self.conn)
        .await
        .map_err(|e| anyhow!("enqueue failed: {e}"))?;
        Ok(job.id.clone())
    }

    async fn load(&self, id: &str) -> Result<Option<JobPayload>> {
        let row = entity::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .map_err(|e| anyhow!("load failed: {e}"))?;
        match row {
            Some(model) => Ok(Some(job_from_data(&model.data)?)),
            None => Ok(None),
        }
    }

    /// Try to claim one ready job. Returns Ok(None) if nothing is ready right now.
    async fn try_claim(&self, queues: &[&str], now: DateTime<Utc>) -> Result<Option<Reserved>> {
        if queues.is_empty() {
            return Ok(None);
        }
        let queue_names: Vec<String> = queues.iter().map(|q| (*q).to_string()).collect();
        let row = entity::Entity::find()
            .filter(entity::Column::Status.eq(STATUS_PENDING))
            .filter(entity::Column::RunAt.lte(millis(now)))
            .filter(entity::Column::Queue.is_in(queue_names))
            .order_by_desc(entity::Column::Priority)
            .order_by_asc(entity::Column::RunAt)
            .one(&self.conn)
            .await
            .map_err(|e| anyhow!("reserve select failed: {e}"))?;

        let Some(model) = row else {
            return Ok(None);
        };
        let id = model.id;
        let mut job: JobPayload = job_from_data(&model.data)?;

        job.attempts += 1;
        job.status = JobStatus::Running;
        let new_data = serde_json::to_string(&job)?;
        let lease_until = now
            + chrono::Duration::from_std(self.visibility_timeout)
                .unwrap_or_else(|_| chrono::Duration::seconds(300));

        let res = entity::Entity::update_many()
            .col_expr(entity::Column::Status, Expr::value(STATUS_RUNNING))
            .col_expr(entity::Column::LockedAt, Expr::value(millis(now)))
            .col_expr(entity::Column::Data, Expr::value(new_data))
            .filter(entity::Column::Id.eq(id.clone()))
            .filter(entity::Column::Status.eq(STATUS_PENDING))
            .exec(&self.conn)
            .await
            .map_err(|e| anyhow!("reserve claim failed: {e}"))?;

        if res.rows_affected == 1 {
            Ok(Some(Reserved { job, lease_until }))
        } else {
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
        entity::Entity::delete_by_id(id)
            .exec(&self.conn)
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
        entity::Entity::update_many()
            .col_expr(entity::Column::Status, Expr::value(STATUS_PENDING))
            .col_expr(entity::Column::RunAt, Expr::value(millis(run_at)))
            .col_expr(
                entity::Column::LockedAt,
                Expr::value(None::<i64>),
            )
            .col_expr(entity::Column::Data, Expr::value(data))
            .filter(entity::Column::Id.eq(id))
            .exec(&self.conn)
            .await
            .map_err(|e| anyhow!("nack failed: {e}"))?;
        Ok(())
    }

    async fn reclaim_expired(&self, queues: &[&str]) -> Result<u64> {
        if queues.is_empty() {
            return Ok(0);
        }
        let cutoff = millis(Utc::now()) - self.visibility_timeout.as_millis() as i64;
        let queue_names: Vec<String> = queues.iter().map(|q| (*q).to_string()).collect();
        let rows = entity::Entity::find()
            .filter(entity::Column::Status.eq(STATUS_RUNNING))
            .filter(entity::Column::LockedAt.lte(cutoff))
            .filter(entity::Column::Queue.is_in(queue_names))
            .all(&self.conn)
            .await
            .map_err(|e| anyhow!("reclaim select failed: {e}"))?;
        let mut count = 0u64;
        for model in rows {
            let id = model.id;
            let Some(mut job) = self.load(&id).await? else {
                continue;
            };
            job.status = JobStatus::Pending;
            job.error = Some("lease expired".to_string());
            let data = serde_json::to_string(&job)?;
            let res = entity::Entity::update_many()
                .col_expr(entity::Column::Status, Expr::value(STATUS_PENDING))
                .col_expr(
                    entity::Column::LockedAt,
                    Expr::value(None::<i64>),
                )
                .col_expr(entity::Column::Data, Expr::value(data))
                .filter(entity::Column::Id.eq(id))
                .filter(entity::Column::Status.eq(STATUS_RUNNING))
                .exec(&self.conn)
                .await
                .map_err(|e| anyhow!("reclaim failed: {e}"))?;
            count += res.rows_affected;
        }
        Ok(count)
    }

    async fn dead_letter(&self, id: &str, reason: &str) -> Result<()> {
        let Some(mut job) = self.load(id).await? else {
            return Ok(());
        };
        job.status = JobStatus::Dead;
        job.error = Some(reason.to_string());
        let data = serde_json::to_string(&job)?;
        entity::Entity::update_many()
            .col_expr(entity::Column::Status, Expr::value(STATUS_DEAD))
            .col_expr(
                entity::Column::LockedAt,
                Expr::value(None::<i64>),
            )
            .col_expr(entity::Column::Data, Expr::value(data))
            .filter(entity::Column::Id.eq(id))
            .exec(&self.conn)
            .await
            .map_err(|e| anyhow!("dead_letter failed: {e}"))?;
        Ok(())
    }

    async fn dead_jobs(&self, queue: &str) -> Result<Vec<JobPayload>> {
        let rows = entity::Entity::find()
            .filter(entity::Column::Status.eq(STATUS_DEAD))
            .filter(entity::Column::Queue.eq(queue))
            .order_by_asc(entity::Column::RunAt)
            .all(&self.conn)
            .await
            .map_err(|e| anyhow!("dead_jobs failed: {e}"))?;
        let mut out = Vec::with_capacity(rows.len());
        for model in rows {
            out.push(job_from_data(&model.data)?);
        }
        Ok(out)
    }

    async fn discard_dead(&self, queue: &str) -> Result<u64> {
        let res = entity::Entity::delete_many()
            .filter(entity::Column::Status.eq(STATUS_DEAD))
            .filter(entity::Column::Queue.eq(queue))
            .exec(&self.conn)
            .await
            .map_err(|e| anyhow!("discard_dead failed: {e}"))?;
        Ok(res.rows_affected)
    }
}
