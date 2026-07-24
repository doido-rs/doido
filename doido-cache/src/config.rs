//! Per-environment cache configuration loaded from the `cache` section of
//! `config/<env>.yml`.
//!
//! [`CacheConfig`] selects the backend (`memory`, `redis`, or `memcache`) and
//! its `endpoint`, and [`CacheConfig::build`] turns that into a live
//! `Arc<dyn CacheStore>`. The Redis and Memcache backends are behind the
//! `cache-redis` / `cache-memcache` cargo features; selecting a backend whose
//! feature is not enabled yields a clear error.

use crate::environment::Environment;
use crate::store::CacheStore;
use crate::{MemoryStore, NamespacedStore};
use doido_core::Result;
use serde::Deserialize;
use std::sync::Arc;

/// Which cache backend to use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheBackend {
    /// In-process `HashMap` + TTL. Single-process; the default.
    #[default]
    Memory,
    /// Redis (`GET`/`SET EX`), distributed. Requires the `cache-redis` feature.
    Redis,
    /// Memcached, distributed. Requires the `cache-memcache` feature.
    Memcache,
}

/// Cache settings, deserialized from the `cache` section of `config/<env>.yml`.
///
/// ```yaml
/// cache:
///   type: redis                       # memory | redis | memcache
///   endpoint: redis://127.0.0.1:6379  # backend address (redis/memcache)
///   namespace: myapp                  # optional key prefix
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CacheConfig {
    /// Backend kind. YAML key is `type`.
    #[serde(default, rename = "type")]
    pub backend: CacheBackend,
    /// Connection address for the `redis`/`memcache` backends. Ignored by
    /// `memory`. Defaults to the backend's standard localhost address.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Optional key prefix applied to every key (via [`NamespacedStore`]).
    #[serde(default)]
    pub namespace: Option<String>,
}

impl CacheConfig {
    /// Builds the configured [`CacheStore`], wrapping it in a [`NamespacedStore`]
    /// when `namespace` is set. Connecting to Redis/Memcached happens here, so
    /// this is async and can fail.
    pub async fn build(&self) -> Result<Arc<dyn CacheStore>> {
        let base: Arc<dyn CacheStore> = match self.backend {
            CacheBackend::Memory => Arc::new(MemoryStore::new()),
            CacheBackend::Redis => self.build_redis().await?,
            CacheBackend::Memcache => self.build_memcache().await?,
        };
        Ok(match &self.namespace {
            Some(prefix) if !prefix.is_empty() => {
                Arc::new(NamespacedStore::new(base, prefix.clone()))
            }
            _ => base,
        })
    }

    #[cfg(feature = "cache-redis")]
    async fn build_redis(&self) -> Result<Arc<dyn CacheStore>> {
        let endpoint = self
            .endpoint
            .clone()
            .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());
        Ok(Arc::new(
            crate::redis_store::RedisStore::connect(&endpoint).await?,
        ))
    }

    #[cfg(not(feature = "cache-redis"))]
    async fn build_redis(&self) -> Result<Arc<dyn CacheStore>> {
        Err(doido_core::anyhow::anyhow!(
            "cache backend 'redis' selected in config but doido-cache was built \
             without the `cache-redis` feature"
        ))
    }

    #[cfg(feature = "cache-memcache")]
    async fn build_memcache(&self) -> Result<Arc<dyn CacheStore>> {
        let endpoint = self
            .endpoint
            .clone()
            .unwrap_or_else(|| "memcache://127.0.0.1:11211".to_string());
        Ok(Arc::new(
            crate::memcache_store::MemcacheStore::connect(endpoint).await?,
        ))
    }

    #[cfg(not(feature = "cache-memcache"))]
    async fn build_memcache(&self) -> Result<Arc<dyn CacheStore>> {
        Err(doido_core::anyhow::anyhow!(
            "cache backend 'memcache' selected in config but doido-cache was \
             built without the `cache-memcache` feature"
        ))
    }
}

/// File-based config deserialized from `config/<env>.yml`. Only the `cache`
/// section is read; other sections (server, database, logger…) are ignored.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct YamlConfig {
    #[serde(default)]
    pub cache: CacheConfig,
}

impl YamlConfig {
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

    /// Parses a [`YamlConfig`] from a YAML string.
    pub fn from_yaml(yaml: &str) -> std::io::Result<Self> {
        serde_norway::from_str(yaml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Loads the current environment's [`CacheConfig`], falling back to the default
/// (in-memory) when the file is missing or has no `cache` section.
pub fn load() -> CacheConfig {
    YamlConfig::load().map(|c| c.cache).unwrap_or_default()
}
