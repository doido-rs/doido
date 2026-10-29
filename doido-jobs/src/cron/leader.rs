use crate::config::{Backend, JobsConfig};
use async_trait::async_trait;
use doido_core::{anyhow::anyhow, Result};
use std::sync::Arc;

/// Distributed lock for a single cron tick so only one worker enqueues.
#[async_trait]
pub trait CronLeaderLock: Send + Sync {
    /// Returns `true` if this worker should enqueue for `slot_key`.
    async fn try_acquire(&self, slot_key: &str) -> Result<bool>;
}

/// No coordination — every worker enqueues (dev / memory / sqlite).
pub struct NoopCronLeader;

#[async_trait]
impl CronLeaderLock for NoopCronLeader {
    async fn try_acquire(&self, _slot_key: &str) -> Result<bool> {
        Ok(true)
    }
}

#[cfg(feature = "jobs-redis")]
const LOCK_TTL_SECS: u64 = 120;

#[cfg(feature = "jobs-redis")]
pub struct RedisCronLeader {
    conn: redis::aio::MultiplexedConnection,
}

#[cfg(feature = "jobs-redis")]
impl RedisCronLeader {
    pub async fn connect(url: &str) -> Result<Self> {
        let client =
            redis::Client::open(url).map_err(|e| anyhow!("cron redis leader: open failed: {e}"))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| anyhow!("cron redis leader: connect failed: {e}"))?;
        Ok(Self { conn })
    }
}

#[cfg(feature = "jobs-redis")]
#[async_trait]
impl CronLeaderLock for RedisCronLeader {
    async fn try_acquire(&self, slot_key: &str) -> Result<bool> {
        let key = format!("{slot_key}:lock");
        let mut conn = self.conn.clone();
        let acquired: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(LOCK_TTL_SECS)
            .query_async(&mut conn)
            .await
            .map_err(|e| anyhow!("cron redis lock failed: {e}"))?;
        Ok(acquired.is_some())
    }
}

#[cfg(feature = "jobs-db")]
enum DbKind {
    Postgres,
    MySql,
    Sqlite,
}

#[cfg(feature = "jobs-db")]
pub struct DbCronLeader {
    conn: doido_model::sea_orm::DatabaseConnection,
}

#[cfg(feature = "jobs-db")]
impl DbCronLeader {
    pub fn new(conn: doido_model::sea_orm::DatabaseConnection) -> Self {
        Self { conn }
    }

    fn db_kind(&self) -> DbKind {
        match self.conn.get_database_backend() {
            doido_model::sea_orm::DatabaseBackend::Postgres => DbKind::Postgres,
            doido_model::sea_orm::DatabaseBackend::MySql => DbKind::MySql,
            doido_model::sea_orm::DatabaseBackend::Sqlite => DbKind::Sqlite,
            _ => DbKind::Sqlite,
        }
    }
}

#[cfg(feature = "jobs-db")]
#[async_trait]
impl CronLeaderLock for DbCronLeader {
    async fn try_acquire(&self, slot_key: &str) -> Result<bool> {
        match self.db_kind() {
            DbKind::Postgres => postgres_try_lock(&self.conn, slot_key).await,
            DbKind::MySql => mysql_try_lock(&self.conn, slot_key).await,
            DbKind::Sqlite => Ok(true),
        }
    }
}

#[cfg(feature = "jobs-db")]
async fn postgres_try_lock(
    conn: &doido_model::sea_orm::DatabaseConnection,
    slot_key: &str,
) -> Result<bool> {
    use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement, Value};

    let row = conn
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_try_advisory_lock(hashtext($1))",
            [Value::from(slot_key.to_string())],
        ))
        .await
        .map_err(|e| anyhow!("postgres cron lock query failed: {e}"))?;
    let Some(row) = row else {
        return Ok(false);
    };
    let acquired: bool = row.try_get_by_index(0).unwrap_or(false);
    Ok(acquired)
}

#[cfg(feature = "jobs-db")]
async fn mysql_try_lock(
    conn: &doido_model::sea_orm::DatabaseConnection,
    slot_key: &str,
) -> Result<bool> {
    use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement, Value};

    let row = conn
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::MySql,
            "SELECT GET_LOCK(?, 0) AS acquired",
            [Value::from(slot_key.to_string())],
        ))
        .await
        .map_err(|e| anyhow!("mysql cron lock query failed: {e}"))?;
    let Some(row) = row else {
        return Ok(false);
    };
    let acquired: i64 = row.try_get_by_index(0).unwrap_or(0);
    Ok(acquired == 1)
}

/// Build a leader lock appropriate for the configured jobs backend.
pub async fn build_cron_leader(config: &JobsConfig) -> Result<Arc<dyn CronLeaderLock>> {
    match config.backend {
        Backend::Redis => build_redis_cron_leader(config).await,
        Backend::Db => build_db_cron_leader(config).await,
        Backend::Memory => {
            doido_core::tracing::warn!(
                "jobs.cron with memory backend has no distributed leader lock; \
                 run a single worker or use db/redis backend"
            );
            Ok(Arc::new(NoopCronLeader))
        }
    }
}

#[cfg(feature = "jobs-redis")]
async fn build_redis_cron_leader(config: &JobsConfig) -> Result<Arc<dyn CronLeaderLock>> {
    let url = config
        .redis_url
        .as_deref()
        .ok_or_else(|| anyhow!("jobs redis backend requires jobs.redis.url"))?;
    let leader = RedisCronLeader::connect(url).await?;
    Ok(Arc::new(leader))
}

#[cfg(not(feature = "jobs-redis"))]
async fn build_redis_cron_leader(_config: &JobsConfig) -> Result<Arc<dyn CronLeaderLock>> {
    Err(anyhow!("redis jobs backend requires jobs-redis feature"))
}

#[cfg(feature = "jobs-db")]
async fn build_db_cron_leader(_config: &JobsConfig) -> Result<Arc<dyn CronLeaderLock>> {
    if let Some(pool) = doido_model::pool::try_pool() {
        if matches!(
            pool.get_database_backend(),
            doido_model::sea_orm::DatabaseBackend::Sqlite
        ) {
            doido_core::tracing::warn!(
                "jobs.cron leader lock is not distributed on SQLite; \
                 run a single worker or use Postgres/MySQL/Redis jobs backend"
            );
            Ok(Arc::new(NoopCronLeader))
        } else {
            Ok(Arc::new(DbCronLeader::new(pool.clone())))
        }
    } else {
        doido_core::tracing::warn!(
            "jobs.cron: database pool not available for leader lock; \
             using noop lock (single worker recommended)"
        );
        Ok(Arc::new(NoopCronLeader))
    }
}

#[cfg(not(feature = "jobs-db"))]
async fn build_db_cron_leader(_config: &JobsConfig) -> Result<Arc<dyn CronLeaderLock>> {
    Err(anyhow!("db jobs backend requires jobs-db feature"))
}
