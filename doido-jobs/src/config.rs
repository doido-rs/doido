//! Config-driven backend selection. Mirrors `doido-cache`'s registry: the engine
//! and CLI only ever see an `Arc<dyn JobQueue>`, never a concrete backend.

use crate::queue::JobQueue;
use crate::worker::EngineConfig;
use doido_core::{anyhow::anyhow, Environment, Result};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    #[default]
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
pub fn build_db_queue(conn: doido_model::sea_orm::DatabaseConnection) -> Arc<dyn JobQueue> {
    Arc::new(crate::db::DbQueue::new(conn))
}

// ── Config-file loading (mirrors `doido-cache`'s `config`) ──────────────────

/// Redis connection settings for the jobs backend (`jobs.redis`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RedisSettings {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
}

/// Jobs settings, deserialized from the `jobs` section of `config/<env>.yml`:
///
/// ```yaml
/// jobs:
///   type: db                 # memory | db | redis
///   queues: [default, mailers]
///   concurrency: 10
///   redis:
///     url: redis://127.0.0.1:6379
///     namespace: myapp:jobs
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct JobsSettings {
    /// Backend kind. YAML key is `type`.
    #[serde(default, rename = "type")]
    pub backend: Backend,
    #[serde(default)]
    pub queues: Option<Vec<String>>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub redis: Option<RedisSettings>,
}

impl JobsSettings {
    /// Turn parsed settings into a runtime [`JobsConfig`], filling unspecified
    /// fields from [`JobsConfig::default`].
    pub fn into_config(self) -> JobsConfig {
        let d = JobsConfig::default();
        let redis = self.redis.unwrap_or_default();
        JobsConfig {
            backend: self.backend,
            queues: self.queues.filter(|q| !q.is_empty()).unwrap_or(d.queues),
            concurrency: self.concurrency.unwrap_or(d.concurrency),
            poll_wait: d.poll_wait,
            reclaim_interval: d.reclaim_interval,
            redis_url: redis.url.or(d.redis_url),
            redis_namespace: redis.namespace.unwrap_or(d.redis_namespace),
        }
    }
}

/// File-based config deserialized from `config/<env>.yml`. Only the `jobs`
/// section is read; other sections are ignored.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct JobsFileConfig {
    #[serde(default)]
    pub jobs: JobsSettings,
}

impl JobsFileConfig {
    /// Loads `config/<env>.yml` for the environment from [`Environment::get_env`].
    pub fn load() -> std::io::Result<Self> {
        Self::load_env(Environment::get_env())
    }

    /// Loads `config/<env>.yml` for a specific environment.
    pub fn load_env(env: Environment) -> std::io::Result<Self> {
        let path = format!("config/{}.yml", env.as_str());
        let contents = std::fs::read_to_string(&path)?;
        Self::from_yaml(&contents)
    }

    /// Parses a [`JobsFileConfig`] from a YAML string.
    pub fn from_yaml(yaml: &str) -> std::io::Result<Self> {
        serde_norway::from_str(yaml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Loads the current environment's [`JobsConfig`], falling back to the default
/// (in-memory) when the file is missing or has no `jobs` section.
pub fn load() -> JobsConfig {
    JobsFileConfig::load()
        .map(|c| c.jobs.into_config())
        .unwrap_or_default()
}

/// Build the configured queue, connecting the database for the `db` backend
/// (which [`build_queue`] cannot construct on its own) and ensuring its table
/// exists. Memory/redis delegate to [`build_queue`].
pub async fn build_configured_queue(config: &JobsConfig) -> Result<Arc<dyn JobQueue>> {
    match config.backend {
        Backend::Db => build_db_backed_queue().await,
        _ => build_queue(config).await,
    }
}

#[cfg(feature = "jobs-db")]
async fn build_db_backed_queue() -> Result<Arc<dyn JobQueue>> {
    // Reuse an already-installed global pool (tests / a booted app), otherwise
    // connect using the app's `database` config.
    let conn = match doido_model::pool::try_pool() {
        Some(pool) => pool.clone(),
        None => doido_model::pool::connect()
            .await
            .map_err(|e| anyhow!("jobs db backend: could not connect to the database: {e}"))?,
    };
    let queue = crate::db::DbQueue::new(conn);
    queue.migrate().await?; // idempotent CREATE TABLE IF NOT EXISTS
    Ok(Arc::new(queue))
}

#[cfg(not(feature = "jobs-db"))]
async fn build_db_backed_queue() -> Result<Arc<dyn JobQueue>> {
    Err(anyhow!(
        "jobs backend 'db' selected in config but doido-jobs was built without the \
         `jobs-db` feature"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jobs_section() {
        let yaml = "jobs:\n  type: db\n  queues: [default, mailers]\n  \
                    concurrency: 10\n  redis:\n    url: redis://x:6379\n    \
                    namespace: app:jobs\n";
        let cfg = JobsFileConfig::from_yaml(yaml).unwrap().jobs.into_config();
        assert_eq!(cfg.backend, Backend::Db);
        assert_eq!(cfg.queues, vec!["default", "mailers"]);
        assert_eq!(cfg.concurrency, 10);
        assert_eq!(cfg.redis_url.as_deref(), Some("redis://x:6379"));
        assert_eq!(cfg.redis_namespace, "app:jobs");
    }

    #[test]
    fn absent_jobs_section_falls_back_to_defaults() {
        let cfg = JobsFileConfig::from_yaml("server:\n  port: 3000\n")
            .unwrap()
            .jobs
            .into_config();
        assert_eq!(cfg.backend, Backend::Memory);
        assert_eq!(cfg.queues, vec!["default"]);
        assert_eq!(cfg.concurrency, JobsConfig::default().concurrency);
    }

    #[test]
    fn backend_deserializes_snake_case() {
        let backend = JobsFileConfig::from_yaml("jobs:\n  type: redis\n")
            .unwrap()
            .jobs
            .backend;
        assert_eq!(backend, Backend::Redis);
    }

    #[tokio::test]
    async fn build_configured_queue_returns_working_memory_queue() {
        use crate::queue::JobPayload;
        let q = build_configured_queue(&JobsConfig::default())
            .await
            .unwrap();
        q.enqueue(JobPayload::new("default", serde_json::json!({}), 1))
            .await
            .unwrap();
        let r = q
            .reserve(&["default"], Duration::from_millis(50))
            .await
            .unwrap();
        assert!(r.is_some());
    }

    #[cfg(not(feature = "jobs-db"))]
    #[tokio::test]
    async fn db_backend_without_feature_errors_clearly() {
        let cfg = JobsConfig {
            backend: Backend::Db,
            ..JobsConfig::default()
        };
        // `Arc<dyn JobQueue>` isn't `Debug`, so extract the error via `.err()`.
        let err = build_configured_queue(&cfg)
            .await
            .err()
            .expect("db backend without the jobs-db feature must error")
            .to_string();
        assert!(err.contains("jobs-db"), "got: {err}");
    }

    #[cfg(feature = "jobs-db")]
    #[tokio::test]
    async fn build_configured_queue_db_over_installed_pool() {
        use crate::queue::JobPayload;
        // This is the only test in the crate that touches the process-global
        // pool, so no lock is needed. Install an in-memory sqlite if absent.
        if doido_model::pool::try_pool().is_none() {
            let conn = doido_model::connect_with_url("sqlite::memory:")
                .await
                .unwrap();
            let _ = doido_model::pool::set_pool(conn);
        }
        let cfg = JobsConfig {
            backend: Backend::Db,
            ..JobsConfig::default()
        };
        let q = build_configured_queue(&cfg).await.unwrap();
        q.enqueue(JobPayload::new("default", serde_json::json!({}), 1))
            .await
            .unwrap();
        let r = q
            .reserve(&["default"], Duration::from_millis(50))
            .await
            .unwrap();
        assert!(r.is_some());
    }
}
