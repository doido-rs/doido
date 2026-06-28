//! Config-driven backend selection. Mirrors `doido-cache`'s registry: the engine
//! and CLI only ever see an `Arc<dyn JobQueue>`, never a concrete backend.

use crate::queue::JobQueue;
use crate::worker::EngineConfig;
use doido_core::{anyhow::anyhow, Result};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Memory,
    Db,
    Redis,
}

impl Backend {
    pub fn parse(s: &str) -> Result<Backend> {
        match s.trim().to_ascii_lowercase().as_str() {
            "memory" | "inmemory" | "in_memory" => Ok(Backend::Memory),
            "db" | "database" | "sql" => Ok(Backend::Db),
            "redis" => Ok(Backend::Redis),
            other => Err(anyhow!("unknown jobs backend: {other}")),
        }
    }
}

/// Runtime configuration for the jobs subsystem (typically loaded from `[jobs]`).
#[derive(Clone, Debug)]
pub struct JobsConfig {
    pub backend: Backend,
    pub queues: Vec<String>,
    pub concurrency: usize,
    pub poll_wait: Duration,
    pub reclaim_interval: Duration,
    pub redis_url: Option<String>,
    pub redis_namespace: String,
}

impl Default for JobsConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Memory,
            queues: vec!["default".to_string()],
            concurrency: 5,
            poll_wait: Duration::from_millis(1000),
            reclaim_interval: Duration::from_secs(30),
            redis_url: None,
            redis_namespace: "doido:jobs".to_string(),
        }
    }
}

impl JobsConfig {
    /// Derive the engine's runtime config from the jobs config.
    pub fn engine_config(&self) -> EngineConfig {
        EngineConfig {
            queues: self.queues.clone(),
            concurrency: self.concurrency.max(1),
            poll_wait: self.poll_wait,
            reclaim_interval: self.reclaim_interval,
        }
    }
}

/// Build the configured queue backend. The `db` backend needs a live database
/// connection, so it is constructed via [`build_db_queue`] instead.
pub async fn build_queue(config: &JobsConfig) -> Result<Arc<dyn JobQueue>> {
    match config.backend {
        Backend::Memory => Ok(Arc::new(crate::memory::MemoryQueue::new())),
        Backend::Redis => build_redis_queue(config).await,
        Backend::Db => Err(anyhow!(
            "the `db` jobs backend must be built with a database connection via build_db_queue()"
        )),
    }
}

#[cfg(feature = "jobs-redis")]
async fn build_redis_queue(config: &JobsConfig) -> Result<Arc<dyn JobQueue>> {
    let url = config
        .redis_url
        .as_deref()
        .ok_or_else(|| anyhow!("redis backend selected but [jobs.redis] url is not set"))?;
    let q = crate::redis::RedisQueue::connect(url, config.redis_namespace.clone()).await?;
    Ok(Arc::new(q))
}

#[cfg(not(feature = "jobs-redis"))]
async fn build_redis_queue(_config: &JobsConfig) -> Result<Arc<dyn JobQueue>> {
    Err(anyhow!(
        "redis backend requires building doido-jobs with the `jobs-redis` feature"
    ))
}

/// Build the database-backed queue from an existing sea-orm connection.
#[cfg(feature = "jobs-db")]
pub fn build_db_queue(
    conn: doido_model::sea_orm::DatabaseConnection,
) -> Arc<dyn JobQueue> {
    Arc::new(crate::db::DbQueue::new(conn))
}
