//! Memcached-backed [`CacheStore`] (feature `cache-memcache`).
//!
//! The `memcache` crate is synchronous, so each operation runs on a blocking
//! task via [`tokio::task::spawn_blocking`]. Values are stored as JSON strings.
//! `increment`/`decrement` are a non-atomic read-modify-write (best-effort, per
//! the v1 cache spec) so a missing key initializes to `0` like the memory store.

use crate::store::CacheStore;
use doido_core::Result;
use serde_json::Value;
use std::sync::Arc;

/// A [`CacheStore`] backed by a Memcached server.
pub struct MemcacheStore {
    client: Arc<memcache::Client>,
}

impl MemcacheStore {
    /// Connects to Memcached at `url` (e.g. `memcache://127.0.0.1:11211`).
    pub async fn connect(url: String) -> Result<Self> {
        let client = tokio::task::spawn_blocking(move || memcache::connect(url.as_str()))
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("memcache connect task panicked: {e}"))?
            .map_err(|e| doido_core::anyhow::anyhow!("memcache connect failed: {e}"))?;
        Ok(Self {
            client: Arc::new(client),
        })
    }
}

#[async_trait::async_trait]
impl CacheStore for MemcacheStore {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        let client = self.client.clone();
        let key = key.to_string();
        let raw: Option<String> = tokio::task::spawn_blocking(move || client.get::<String>(&key))
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("memcache task panicked: {e}"))?
            .map_err(|e| doido_core::anyhow::anyhow!("memcache get failed: {e}"))?;
        match raw {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()> {
        let client = self.client.clone();
        let key = key.to_string();
        let payload = serde_json::to_string(&value)?;
        // Memcached expirations are u32 seconds (0 = never expire).
        let expiration = ttl_secs.unwrap_or(0).min(u32::MAX as u64) as u32;
        tokio::task::spawn_blocking(move || client.set(&key, payload.as_str(), expiration))
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("memcache task panicked: {e}"))?
            .map_err(|e| doido_core::anyhow::anyhow!("memcache set failed: {e}"))?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let client = self.client.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || client.delete(&key))
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("memcache task panicked: {e}"))?
            .map_err(|e| doido_core::anyhow::anyhow!("memcache delete failed: {e}"))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }

    async fn increment(&self, key: &str, by: i64) -> Result<i64> {
        let current = self.get(key).await?.and_then(|v| v.as_i64()).unwrap_or(0);
        let new_value = current + by;
        self.set(key, serde_json::json!(new_value), None).await?;
        Ok(new_value)
    }

    async fn decrement(&self, key: &str, by: i64) -> Result<i64> {
        self.increment(key, -by).await
    }

    async fn clear(&self) -> Result<()> {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || client.flush())
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("memcache task panicked: {e}"))?
            .map_err(|e| doido_core::anyhow::anyhow!("memcache flush failed: {e}"))?;
        Ok(())
    }
}
