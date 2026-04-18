use serde_json::Value;
use doido_core::Result;

#[async_trait::async_trait]
pub trait CacheStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>>;
    async fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn increment(&self, key: &str, by: i64) -> Result<i64>;
    async fn decrement(&self, key: &str, by: i64) -> Result<i64>;
    async fn clear(&self) -> Result<()>;
}
