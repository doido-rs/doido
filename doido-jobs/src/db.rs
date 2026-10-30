//! Database-backed queue (sea-orm). Enabled with the `jobs-db` feature.
//!
//! Jobs live in a single `doido_jobs` table. On **MySQL** and **PostgreSQL** (when the
//! corresponding crate features are enabled), `reserve` claims a row inside a transaction
//! using `SELECT … FOR UPDATE SKIP LOCKED`, so a single worker process can reserve up to
//! `jobs.concurrency` distinct jobs in parallel. On **SQLite** and other builds without
//! those features, claim uses a guarded update (select + `UPDATE … WHERE status IN
//! (pending, failed)`), which serializes on the same row under contention but needs no
//! row-level skip lock support.

use crate::entity;
use crate::inspect::{
    encode_job_page_token, JobBucket, JobListPage, JobPageAnchor, JobPageDirection, JobSortKey,
    QueueCounts,
};
use crate::queue::{JobId, JobPayload, JobQueue, JobStatus, Reserved};
use chrono::{DateTime, Utc};
use doido_core::{anyhow::anyhow, Result};
use doido_model::migration::{
    add_column_with_opts, add_index_with_opts, create_table_with_opts, CmdOpts,
};
use doido_model::sea_orm::sea_query::Expr;
#[cfg(any(feature = "mysql", feature = "postgres"))]
use doido_model::sea_orm::sea_query::{LockBehavior, LockType};
#[cfg(any(feature = "mysql", feature = "postgres"))]
use doido_model::sea_orm::TransactionTrait;
use doido_model::sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, NotSet,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use doido_model::SchemaManager;
use std::time::Duration;

const STATUS_PENDING: &str = "pending";
const STATUS_RUNNING: &str = "running";
const STATUS_FAILED: &str = "failed";
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

fn list_jobs_base_filter(
    queue: &str,
    bucket: JobBucket,
    now: i64,
) -> doido_model::sea_orm::Select<entity::Entity> {
    let mut q = entity::Entity::find().filter(entity::Column::Queue.eq(queue));
    match bucket {
        JobBucket::PendingReady => {
            q = q
                .filter(entity::Column::Status.eq(STATUS_PENDING))
                .filter(entity::Column::RunAt.lte(now));
        }
        JobBucket::Scheduled => {
            q = q
                .filter(entity::Column::Status.eq(STATUS_PENDING))
                .filter(entity::Column::RunAt.gt(now));
        }
        JobBucket::Running => {
            q = q.filter(entity::Column::Status.eq(STATUS_RUNNING));
        }
        JobBucket::Retry => {
            q = q
                .filter(entity::Column::Status.eq(STATUS_FAILED))
                .filter(entity::Column::RunAt.gt(now));
        }
        JobBucket::FailedReady => {
            q = q
                .filter(entity::Column::Status.eq(STATUS_FAILED))
                .filter(entity::Column::RunAt.lte(now));
        }
        JobBucket::Dead => {
            q = q.filter(entity::Column::Status.eq(STATUS_DEAD));
        }
        JobBucket::InQueue => {
            q = q.filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(entity::Column::Status.eq(STATUS_PENDING))
                            .add(entity::Column::RunAt.lte(now)),
                    )
                    .add(
                        Condition::all()
                            .add(entity::Column::Status.eq(STATUS_FAILED))
                            .add(entity::Column::RunAt.lte(now)),
                    ),
            );
        }
    }
    q
}

fn after_key_condition(key: &JobSortKey) -> Condition {
    Condition::any()
        .add(entity::Column::Priority.lt(key.priority))
        .add(
            Condition::all()
                .add(entity::Column::Priority.eq(key.priority))
                .add(entity::Column::RunAt.gt(key.run_at_ms)),
        )
        .add(
            Condition::all()
                .add(entity::Column::Priority.eq(key.priority))
                .add(entity::Column::RunAt.eq(key.run_at_ms))
                .add(entity::Column::JobId.gt(key.id.clone())),
        )
}

fn before_key_condition(key: &JobSortKey) -> Condition {
    Condition::any()
        .add(entity::Column::Priority.gt(key.priority))
        .add(
            Condition::all()
                .add(entity::Column::Priority.eq(key.priority))
                .add(entity::Column::RunAt.lt(key.run_at_ms)),
        )
        .add(
            Condition::all()
                .add(entity::Column::Priority.eq(key.priority))
                .add(entity::Column::RunAt.eq(key.run_at_ms))
                .add(entity::Column::JobId.lt(key.id.clone())),
        )
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
        let manager = SchemaManager::new(&self.conn);
        let if_not_exists = CmdOpts::if_not_exists();
        create_table_with_opts(
            &manager,
            "doido_jobs",
            |t| {
                t.string("job_id").not_null().unique_key();
                t.string("queue").not_null();
                t.string("status").not_null();
                t.big_integer("priority");
                t.big_integer("run_at").not_null();
                t.big_integer("locked_at");
                t.text("data").not_null();
                t.text("last_error");
            },
            if_not_exists,
        )
        .await
        .map_err(|e| anyhow!("doido_jobs migrate failed: {e}"))?;
        add_column_with_opts(
            &manager,
            "doido_jobs",
            "last_error",
            |c| {
                c.text();
            },
            if_not_exists,
        )
        .await
        .map_err(|e| anyhow!("doido_jobs last_error column failed: {e}"))?;
        add_index_with_opts(
            &manager,
            "doido_jobs",
            &["queue", "status", "run_at"],
            if_not_exists,
        )
        .await
        .map_err(|e| anyhow!("doido_jobs index failed: {e}"))?;
        Ok(())
    }

    async fn insert(&self, job: &JobPayload) -> Result<JobId> {
        let data = serde_json::to_string(job)?;
        entity::ActiveModel {
            id: NotSet,
            job_id: Set(job.id.clone()),
            queue: Set(job.queue.clone()),
            status: Set(STATUS_PENDING.to_string()),
            priority: Set(job.priority as i64),
            run_at: Set(millis(job.run_at)),
            locked_at: Set(None),
            data: Set(data),
            last_error: Set(None),
        }
        .insert(&self.conn)
        .await
        .map_err(|e| anyhow!("enqueue failed: {e}"))?;
        Ok(job.id.clone())
    }

    async fn load(&self, job_id: &str) -> Result<Option<JobPayload>> {
        let row = entity::Entity::find()
            .filter(entity::Column::JobId.eq(job_id))
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
        #[cfg(any(feature = "mysql", feature = "postgres"))]
        {
            return self.try_claim_skip_locked(queues, now).await;
        }
        #[cfg(not(any(feature = "mysql", feature = "postgres")))]
        {
            return self.try_claim_legacy(queues, now).await;
        }
    }

    #[cfg(not(any(feature = "mysql", feature = "postgres")))]
    async fn try_claim_legacy(
        &self,
        queues: &[&str],
        now: DateTime<Utc>,
    ) -> Result<Option<Reserved>> {
        let queue_names: Vec<String> = queues.iter().map(|q| (*q).to_string()).collect();
        let row = entity::Entity::find()
            .filter(entity::Column::Status.is_in([STATUS_PENDING, STATUS_FAILED]))
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
        let row_id = model.id;
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
            .col_expr(entity::Column::LastError, Expr::value(None::<String>))
            .filter(entity::Column::Id.eq(row_id))
            .filter(entity::Column::Status.is_in([STATUS_PENDING, STATUS_FAILED]))
            .exec(&self.conn)
            .await
            .map_err(|e| anyhow!("reserve claim failed: {e}"))?;

        if res.rows_affected == 1 {
            Ok(Some(Reserved { job, lease_until }))
        } else {
            Ok(None)
        }
    }

    #[cfg(any(feature = "mysql", feature = "postgres"))]
    async fn try_claim_skip_locked(
        &self,
        queues: &[&str],
        now: DateTime<Utc>,
    ) -> Result<Option<Reserved>> {
        let queue_names: Vec<String> = queues.iter().map(|q| (*q).to_string()).collect();
        let txn = self
            .conn
            .begin()
            .await
            .map_err(|e| anyhow!("reserve begin txn failed: {e}"))?;

        let row = entity::Entity::find()
            .filter(entity::Column::Status.is_in([STATUS_PENDING, STATUS_FAILED]))
            .filter(entity::Column::RunAt.lte(millis(now)))
            .filter(entity::Column::Queue.is_in(queue_names))
            .order_by_desc(entity::Column::Priority)
            .order_by_asc(entity::Column::RunAt)
            .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
            .one(&txn)
            .await
            .map_err(|e| anyhow!("reserve select failed: {e}"))?;

        let Some(model) = row else {
            txn.rollback()
                .await
                .map_err(|e| anyhow!("reserve rollback failed: {e}"))?;
            return Ok(None);
        };

        let row_id = model.id;
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
            .col_expr(entity::Column::LastError, Expr::value(None::<String>))
            .filter(entity::Column::Id.eq(row_id))
            .filter(entity::Column::Status.is_in([STATUS_PENDING, STATUS_FAILED]))
            .exec(&txn)
            .await
            .map_err(|e| anyhow!("reserve claim failed: {e}"))?;

        if res.rows_affected != 1 {
            txn.rollback()
                .await
                .map_err(|e| anyhow!("reserve rollback failed: {e}"))?;
            return Ok(None);
        }

        txn.commit()
            .await
            .map_err(|e| anyhow!("reserve commit failed: {e}"))?;
        Ok(Some(Reserved { job, lease_until }))
    }

    async fn list_jobs_forward(
        &self,
        base: doido_model::sea_orm::Select<entity::Entity>,
        limit: u32,
        page: Option<JobPageAnchor>,
    ) -> Result<JobListPage> {
        let mut q = base;
        let start_after = match page {
            Some(JobPageAnchor::After(key)) => {
                q = q.filter(after_key_condition(&key));
                Some(key)
            }
            _ => None,
        };
        let rows = q
            .order_by_desc(entity::Column::Priority)
            .order_by_asc(entity::Column::RunAt)
            .order_by_asc(entity::Column::JobId)
            .limit((limit as u64) + 1)
            .all(&self.conn)
            .await
            .map_err(|e| anyhow!("list_jobs failed: {e}"))?;
        let mut jobs = Vec::with_capacity(rows.len());
        for model in rows {
            jobs.push(job_from_data(&model.data)?);
        }
        let has_next = jobs.len() > limit as usize;
        if has_next {
            jobs.pop();
        }
        let has_prev = start_after.is_some();
        Ok(job_list_page_from_jobs(jobs, has_prev, has_next))
    }

    async fn list_jobs_before(
        &self,
        base: doido_model::sea_orm::Select<entity::Entity>,
        limit: u32,
        before: &JobSortKey,
    ) -> Result<JobListPage> {
        let rows = base
            .clone()
            .filter(before_key_condition(before))
            .order_by_asc(entity::Column::Priority)
            .order_by_desc(entity::Column::RunAt)
            .order_by_desc(entity::Column::JobId)
            .limit(limit as u64)
            .all(&self.conn)
            .await
            .map_err(|e| anyhow!("list_jobs before failed: {e}"))?;
        let mut jobs = Vec::with_capacity(rows.len());
        for model in rows {
            jobs.push(job_from_data(&model.data)?);
        }
        jobs.reverse();
        let has_next = base
            .clone()
            .filter(at_or_after_key_condition(before))
            .count(&self.conn)
            .await
            .map_err(|e| anyhow!("list_jobs count next: {e}"))?
            > 0;
        let has_prev = if let Some(first) = jobs.first() {
            let key = JobSortKey::from_job(first);
            base.filter(before_key_condition(&key))
                .count(&self.conn)
                .await
                .map_err(|e| anyhow!("list_jobs count prev: {e}"))?
                > 0
        } else {
            false
        };
        Ok(job_list_page_from_jobs(jobs, has_prev, has_next))
    }
}

fn at_or_after_key_condition(key: &JobSortKey) -> Condition {
    Condition::any()
        .add(entity::Column::Priority.lt(key.priority))
        .add(
            Condition::all()
                .add(entity::Column::Priority.eq(key.priority))
                .add(entity::Column::RunAt.gt(key.run_at_ms)),
        )
        .add(
            Condition::all()
                .add(entity::Column::Priority.eq(key.priority))
                .add(entity::Column::RunAt.eq(key.run_at_ms))
                .add(entity::Column::JobId.gte(key.id.clone())),
        )
}

fn job_list_page_from_jobs(jobs: Vec<JobPayload>, has_prev: bool, has_next: bool) -> JobListPage {
    let next_page_token = if has_next {
        jobs.last()
            .map(|j| encode_job_page_token(&JobSortKey::from_job(j), JobPageDirection::After))
    } else {
        None
    };
    let prev_page_token = if has_prev {
        jobs.first()
            .map(|j| encode_job_page_token(&JobSortKey::from_job(j), JobPageDirection::Before))
    } else {
        None
    };
    JobListPage {
        jobs,
        next_page_token,
        prev_page_token,
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

    async fn ack(&self, job_id: &str) -> Result<()> {
        entity::Entity::delete_many()
            .filter(entity::Column::JobId.eq(job_id))
            .exec(&self.conn)
            .await
            .map_err(|e| anyhow!("ack failed: {e}"))?;
        Ok(())
    }

    async fn nack(&self, job_id: &str, retry_at: Option<DateTime<Utc>>, error: &str) -> Result<()> {
        let Some(mut job) = self.load(job_id).await? else {
            return Ok(());
        };
        let run_at = retry_at.unwrap_or_else(Utc::now);
        job.status = JobStatus::Failed;
        job.error = Some(error.to_string());
        job.run_at = run_at;
        let data = serde_json::to_string(&job)?;
        entity::Entity::update_many()
            .col_expr(entity::Column::Status, Expr::value(STATUS_FAILED))
            .col_expr(entity::Column::RunAt, Expr::value(millis(run_at)))
            .col_expr(entity::Column::LockedAt, Expr::value(None::<i64>))
            .col_expr(entity::Column::Data, Expr::value(data))
            .col_expr(entity::Column::LastError, Expr::value(error.to_string()))
            .filter(entity::Column::JobId.eq(job_id))
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
            let row_id = model.id;
            let mut job = match job_from_data(&model.data) {
                Ok(j) => j,
                Err(_) => continue,
            };
            job.status = JobStatus::Pending;
            let reason = "lease expired";
            job.error = Some(reason.to_string());
            let data = serde_json::to_string(&job)?;
            let res = entity::Entity::update_many()
                .col_expr(entity::Column::Status, Expr::value(STATUS_PENDING))
                .col_expr(entity::Column::LockedAt, Expr::value(None::<i64>))
                .col_expr(entity::Column::Data, Expr::value(data))
                .col_expr(entity::Column::LastError, Expr::value(reason))
                .filter(entity::Column::Id.eq(row_id))
                .filter(entity::Column::Status.eq(STATUS_RUNNING))
                .exec(&self.conn)
                .await
                .map_err(|e| anyhow!("reclaim failed: {e}"))?;
            count += res.rows_affected;
        }
        Ok(count)
    }

    async fn dead_letter(&self, job_id: &str, reason: &str) -> Result<()> {
        let Some(mut job) = self.load(job_id).await? else {
            return Ok(());
        };
        job.status = JobStatus::Dead;
        job.error = Some(reason.to_string());
        let data = serde_json::to_string(&job)?;
        entity::Entity::update_many()
            .col_expr(entity::Column::Status, Expr::value(STATUS_DEAD))
            .col_expr(entity::Column::LockedAt, Expr::value(None::<i64>))
            .col_expr(entity::Column::Data, Expr::value(data))
            .col_expr(entity::Column::LastError, Expr::value(reason.to_string()))
            .filter(entity::Column::JobId.eq(job_id))
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

    async fn queue_counts(&self, queues: &[&str]) -> Result<Vec<(String, QueueCounts)>> {
        let now = millis(Utc::now());
        let mut out = Vec::with_capacity(queues.len());
        for name in queues {
            let queue = (*name).to_string();
            let pending_ready = entity::Entity::find()
                .filter(entity::Column::Queue.eq(&queue))
                .filter(entity::Column::Status.eq(STATUS_PENDING))
                .filter(entity::Column::RunAt.lte(now))
                .count(&self.conn)
                .await
                .map_err(|e| anyhow!("count pending_ready: {e}"))?;
            let scheduled = entity::Entity::find()
                .filter(entity::Column::Queue.eq(&queue))
                .filter(entity::Column::Status.eq(STATUS_PENDING))
                .filter(entity::Column::RunAt.gt(now))
                .count(&self.conn)
                .await
                .map_err(|e| anyhow!("count scheduled: {e}"))?;
            let running = entity::Entity::find()
                .filter(entity::Column::Queue.eq(&queue))
                .filter(entity::Column::Status.eq(STATUS_RUNNING))
                .count(&self.conn)
                .await
                .map_err(|e| anyhow!("count running: {e}"))?;
            let retry = entity::Entity::find()
                .filter(entity::Column::Queue.eq(&queue))
                .filter(entity::Column::Status.eq(STATUS_FAILED))
                .filter(entity::Column::RunAt.gt(now))
                .count(&self.conn)
                .await
                .map_err(|e| anyhow!("count retry: {e}"))?;
            let failed_ready = entity::Entity::find()
                .filter(entity::Column::Queue.eq(&queue))
                .filter(entity::Column::Status.eq(STATUS_FAILED))
                .filter(entity::Column::RunAt.lte(now))
                .count(&self.conn)
                .await
                .map_err(|e| anyhow!("count failed_ready: {e}"))?;
            let dead = entity::Entity::find()
                .filter(entity::Column::Queue.eq(&queue))
                .filter(entity::Column::Status.eq(STATUS_DEAD))
                .count(&self.conn)
                .await
                .map_err(|e| anyhow!("count dead: {e}"))?;
            out.push((
                queue,
                QueueCounts {
                    pending_ready: pending_ready as u32,
                    scheduled: scheduled as u32,
                    running: running as u32,
                    retry: retry as u32,
                    failed_ready: failed_ready as u32,
                    dead: dead as u32,
                },
            ));
        }
        Ok(out)
    }

    async fn list_jobs(
        &self,
        queue: &str,
        bucket: JobBucket,
        limit: u32,
        page: Option<JobPageAnchor>,
    ) -> Result<JobListPage> {
        let now = millis(Utc::now());
        let base = list_jobs_base_filter(queue, bucket, now);
        let limit = limit.max(1);
        match page {
            None | Some(JobPageAnchor::After(_)) => self.list_jobs_forward(base, limit, page).await,
            Some(JobPageAnchor::Before(key)) => self.list_jobs_before(base, limit, &key).await,
        }
    }

    async fn retry_dead_job(&self, queue: &str, job_id: &str) -> Result<()> {
        let dead = self.dead_jobs(queue).await?;
        let Some(job) = dead.into_iter().find(|j| j.id == job_id) else {
            return Ok(());
        };
        if self.discard_dead_job(queue, job_id).await? {
            self.enqueue(job.requeue_fresh()).await?;
        }
        Ok(())
    }

    async fn discard_dead_job(&self, queue: &str, job_id: &str) -> Result<bool> {
        let res = entity::Entity::delete_many()
            .filter(entity::Column::Queue.eq(queue))
            .filter(entity::Column::Status.eq(STATUS_DEAD))
            .filter(entity::Column::JobId.eq(job_id))
            .exec(&self.conn)
            .await
            .map_err(|e| anyhow!("discard_dead_job failed: {e}"))?;
        Ok(res.rows_affected > 0)
    }
}
