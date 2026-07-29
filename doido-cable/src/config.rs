//! Config-driven pub/sub selection for the cable server, read from the `cable`
//! section of `config/<env>.yml`. Mirrors the `config` modules in
//! `doido-jobs`/`doido-cache`: callers only ever see an `Arc<dyn PubSub>`.
//!
//! ```yaml
//! cable:
//!   type: redis           # memory (default) | redis | db
//!   ping_interval: 3      # heartbeat seconds
//!   mount_path: /cable
//!   redis:
//!     url: redis://127.0.0.1:6379
//! ```

use crate::pubsub::PubSub;
use doido_core::{anyhow::anyhow, Environment, Result};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

/// Which pub/sub backend broadcasts reach subscribers through. YAML key `type`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    #[default]
    Memory,
    Redis,
    Db,
}

/// Runtime cable configuration.
#[derive(Clone, Debug)]
pub struct CableConfig {
    pub backend: Backend,
    /// How often the server sends an ActionCable `ping`.
    pub ping_interval: Duration,
    /// Path the cable endpoint is mounted at.
    pub mount_path: String,
    pub redis_url: Option<String>,
}

impl Default for CableConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Memory,
            ping_interval: Duration::from_secs(3),
            mount_path: "/cable".to_string(),
            redis_url: None,
        }
    }
}

/// Build the configured pub/sub backend. The `db` backend needs a live database
/// connection, so it is built via [`build_configured_pubsub`] instead.
pub async fn build_pubsub(config: &CableConfig) -> Result<Arc<dyn PubSub>> {
    match config.backend {
        Backend::Memory => Ok(Arc::new(crate::pubsub::MemoryPubSub::new())),
        Backend::Redis => build_redis_pubsub(config).await,
        Backend::Db => Err(anyhow!(
            "the `db` cable backend must be built with a database connection via \
             build_configured_pubsub()"
        )),
    }
}

/// Build the configured pub/sub, connecting the database for the `db` backend
/// (which [`build_pubsub`] cannot construct on its own). Memory/redis delegate.
pub async fn build_configured_pubsub(config: &CableConfig) -> Result<Arc<dyn PubSub>> {
    match config.backend {
        Backend::Db => build_db_pubsub().await,
        _ => build_pubsub(config).await,
    }
}

#[cfg(feature = "cable-redis")]
async fn build_redis_pubsub(config: &CableConfig) -> Result<Arc<dyn PubSub>> {
    let url = config
        .redis_url
        .as_deref()
        .ok_or_else(|| anyhow!("redis cable backend selected but [cable.redis] url is not set"))?;
    Ok(Arc::new(
        crate::redis_pubsub::RedisPubSub::connect(url).await?,
    ))
}

#[cfg(not(feature = "cable-redis"))]
async fn build_redis_pubsub(_config: &CableConfig) -> Result<Arc<dyn PubSub>> {
    Err(anyhow!(
        "redis cable backend requires building doido-cable with the `cable-redis` feature"
    ))
}

#[cfg(feature = "cable-db")]
async fn build_db_pubsub() -> Result<Arc<dyn PubSub>> {
    let conn = match doido_model::pool::try_pool() {
        Some(pool) => pool.clone(),
        None => doido_model::pool::connect()
            .await
            .map_err(|e| anyhow!("cable db backend: could not connect to the database: {e}"))?,
    };
    Ok(Arc::new(crate::db_pubsub::DbPubSub::connect(conn).await?))
}

#[cfg(not(feature = "cable-db"))]
async fn build_db_pubsub() -> Result<Arc<dyn PubSub>> {
    Err(anyhow!(
        "db cable backend requires building doido-cable with the `cable-db` feature"
    ))
}

// ── Config-file loading (mirrors `doido-jobs`'s `config`) ────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RedisSettings {
    #[serde(default)]
    pub url: Option<String>,
}

/// Parsed `cable` section, before defaults are applied.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CableSettings {
    #[serde(default, rename = "type")]
    pub backend: Backend,
    #[serde(default)]
    pub ping_interval: Option<u64>,
    #[serde(default)]
    pub mount_path: Option<String>,
    #[serde(default)]
    pub redis: Option<RedisSettings>,
}

impl CableSettings {
    pub fn into_config(self) -> CableConfig {
        let d = CableConfig::default();
        CableConfig {
            backend: self.backend,
            ping_interval: self
                .ping_interval
                .filter(|s| *s > 0)
                .map(Duration::from_secs)
                .unwrap_or(d.ping_interval),
            mount_path: self.mount_path.unwrap_or(d.mount_path),
            redis_url: self.redis.and_then(|r| r.url).or(d.redis_url),
        }
    }
}

/// File config deserialized from `config/<env>.yml`; only `cable` is read.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CableFileConfig {
    #[serde(default)]
    pub cable: CableSettings,
}

impl CableFileConfig {
    pub fn load() -> std::io::Result<Self> {
        Self::load_env(Environment::get_env())
    }

    pub fn load_env(env: Environment) -> std::io::Result<Self> {
        let path = format!("config/{}.yml", env.as_str());
        let contents = std::fs::read_to_string(&path)?;
        Self::from_yaml(&contents)
    }

    pub fn from_yaml(yaml: &str) -> std::io::Result<Self> {
        serde_norway::from_str(yaml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Load the current environment's [`CableConfig`], falling back to the default
/// (in-memory) when the file is missing or has no `cable` section.
pub fn load() -> CableConfig {
    CableFileConfig::load()
        .map(|c| c.cable.into_config())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cable_section() {
        let yaml = "cable:\n  type: redis\n  ping_interval: 10\n  mount_path: /ws\n  \
                    redis:\n    url: redis://x:6379\n";
        let cfg = CableFileConfig::from_yaml(yaml)
            .unwrap()
            .cable
            .into_config();
        assert_eq!(cfg.backend, Backend::Redis);
        assert_eq!(cfg.ping_interval, Duration::from_secs(10));
        assert_eq!(cfg.mount_path, "/ws");
        assert_eq!(cfg.redis_url.as_deref(), Some("redis://x:6379"));
    }

    #[test]
    fn absent_section_falls_back_to_memory_defaults() {
        let cfg = CableFileConfig::from_yaml("server:\n  port: 3000\n")
            .unwrap()
            .cable
            .into_config();
        assert_eq!(cfg.backend, Backend::Memory);
        assert_eq!(cfg.ping_interval, Duration::from_secs(3));
        assert_eq!(cfg.mount_path, "/cable");
    }
}
