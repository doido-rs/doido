//! Redis-backed [`CacheStore`] (feature `cache-redis`).
//!
//! Values are stored as JSON strings (optionally gzip-compressed with the
//! [`crate::codec`] prefix). `increment`/`decrement` use Redis `INCRBY`/`DECRBY`,
//! which operate on integer-encoded values (a JSON integer encodes identically).

use crate::codec;
use crate::store::CacheStore;
use doido_core::Result;
use redis::AsyncCommands;
use serde_json::Value;

/// A [`CacheStore`] backed by a Redis server over a shared multiplexed
/// connection.
pub struct RedisStore {
    conn: redis::aio::MultiplexedConnection,
    compress: bool,
}

impl RedisStore {
    /// Connects to Redis at `url` (e.g. `redis://127.0.0.1:6379`).
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with_options(url, false).await
    }

    /// Connects to Redis with optional gzip compression for stored values.
    pub async fn connect_with_options(url: &str, compress: bool) -> Result<Self> {
        let client = redis::Client::open(url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self { conn, compress })
    }

    fn decode_raw(raw: &str) -> Result<Value> {
        codec::unpack(raw)
    }

    fn encode_value(&self, value: &Value) -> Result<String> {
        codec::pack(value, self.compress)
    }
}

#[async_trait::async_trait]
impl CacheStore for RedisStore {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        let mut conn = self.conn.clone();
        let raw: Option<String> = conn.get(key).await?;
        match raw {
            Some(s) => Ok(Some(Self::decode_raw(&s)?)),
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()> {
        let mut conn = self.conn.clone();
        let payload = self.encode_value(&value)?;
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

    async fn read_many(&self, keys: &[&str]) -> Result<Vec<Option<Value>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let mut conn = self.conn.clone();
        let raw: Vec<Option<String>> = conn.mget(keys).await?;
        raw.into_iter()
            .map(|opt| opt.map(|s| Self::decode_raw(&s)).transpose())
            .collect()
    }
}
