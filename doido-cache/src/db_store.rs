//! Database-backed cache store (feature `cache-db`) — the Solid Cache analogue,
//! so caching needs no separate service. Entries live in a `cache_entries` table
//! with an optional expiry.

use crate::store::CacheStore;
use doido_core::Result;
use doido_model::sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value as DbValue,
};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// A [`CacheStore`] backed by a database table.
pub struct DbCacheStore {
    conn: DatabaseConnection,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl DbCacheStore {
    /// Connect and ensure the `cache_entries` table exists.
    pub async fn connect(conn: DatabaseConnection) -> Result<Self> {
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS cache_entries (\
                key TEXT PRIMARY KEY, value TEXT NOT NULL, expires_at INTEGER)",
        )
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("cache_entries migrate failed: {e}"))?;
        Ok(Self { conn })
    }
}

#[async_trait::async_trait]
impl CacheStore for DbCacheStore {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT value, expires_at FROM cache_entries WHERE key = ?",
            [DbValue::from(key)],
        );
        let row = self
            .conn
            .query_one_raw(stmt)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("cache get failed: {e}"))?;
        let Some(row) = row else { return Ok(None) };
        let expires_at: Option<i64> = row.try_get("", "expires_at").ok();
        if let Some(exp) = expires_at {
            if exp <= now_secs() {
                self.delete(key).await?;
                return Ok(None);
            }
        }
        let value: String = row
            .try_get("", "value")
            .map_err(|e| doido_core::anyhow::anyhow!("cache value: {e}"))?;
        Ok(serde_json::from_str(&value).ok())
    }

    async fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()> {
        let expires_at = ttl_secs.map(|t| now_secs() + t as i64);
        let json = serde_json::to_string(&value).unwrap_or_default();
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO cache_entries (key, value, expires_at) VALUES (?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, expires_at = excluded.expires_at",
            [
                DbValue::from(key),
                DbValue::from(json),
                DbValue::from(expires_at),
            ],
        );
        self.conn
            .execute_raw(stmt)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("cache set failed: {e}"))?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "DELETE FROM cache_entries WHERE key = ?",
            [DbValue::from(key)],
        );
        self.conn
            .execute_raw(stmt)
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("cache delete failed: {e}"))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }

    async fn increment(&self, key: &str, by: i64) -> Result<i64> {
        let current = self.get(key).await?.and_then(|v| v.as_i64()).unwrap_or(0);
        let next = current + by;
        self.set(key, Value::from(next), None).await?;
        Ok(next)
    }

    async fn decrement(&self, key: &str, by: i64) -> Result<i64> {
        self.increment(key, -by).await
    }

    async fn clear(&self) -> Result<()> {
        self.conn
            .execute_unprepared("DELETE FROM cache_entries")
            .await
            .map_err(|e| doido_core::anyhow::anyhow!("cache clear failed: {e}"))?;
        Ok(())
    }
}
