//! Redis-backed [`CacheStore`] (feature `cache-redis`).
//!
//! Values are stored as JSON strings, so `get`/`set` round-trip any
//! `serde_json::Value`. `increment`/`decrement` use Redis `INCRBY`/`DECRBY`,
//! which operate on integer-encoded values (a JSON integer encodes identically).

use crate::store::CacheStore;
use doido_core::Result;
use redis::AsyncCommands;
use serde_json::Value;

/// A [`CacheStore`] backed by a Redis server over a shared multiplexed
/// connection.
pub struct RedisStore {
    conn: redis::aio::MultiplexedConnection,
}

impl RedisStore {
    /// Connects to Redis at `url` (e.g. `redis://127.0.0.1:6379`).
    pub async fn connect(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self { conn })
    }
}

#[async_trait::async_trait]
impl CacheStore for RedisStore {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        let mut conn = self.conn.clone();
        let raw: Option<String> = conn.get(key).await?;
        match raw {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()> {
        let mut conn = self.conn.clone();
        let payload = serde_json::to_string(&value)?;
        match ttl_secs {
            Some(ttl) => {
                let _: () = conn.set_ex(key, payload, ttl).await?;
            }
            None => {
                let _: () = conn.set(key, payload).await?;
            }
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.conn.clone();
        let _: () = conn.del(key).await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.conn.clone();
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    async fn increment(&self, key: &str, by: i64) -> Result<i64> {
        let mut conn = self.conn.clone();
        let value: i64 = conn.incr(key, by).await?;
        Ok(value)
    }

    async fn decrement(&self, key: &str, by: i64) -> Result<i64> {
        let mut conn = self.conn.clone();
        let value: i64 = conn.decr(key, by).await?;
        Ok(value)
    }

    async fn clear(&self) -> Result<()> {
        let mut conn = self.conn.clone();
        let _: () = redis::cmd("FLUSHDB").query_async(&mut conn).await?;
        Ok(())
    }
}
