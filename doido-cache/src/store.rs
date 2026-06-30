use doido_core::Result;
use serde_json::Value;
use std::sync::Arc;

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

/// Lets a type-erased `Arc<dyn CacheStore>` be used wherever a `CacheStore` is
/// expected — notably so [`crate::NamespacedStore`] can wrap a backend selected
/// at runtime from config.
#[async_trait::async_trait]
impl CacheStore for Arc<dyn CacheStore> {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        (**self).get(key).await
    }
    async fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()> {
        (**self).set(key, value, ttl_secs).await
    }
    async fn delete(&self, key: &str) -> Result<()> {
        (**self).delete(key).await
    }
    async fn exists(&self, key: &str) -> Result<bool> {
        (**self).exists(key).await
    }
    async fn increment(&self, key: &str, by: i64) -> Result<i64> {
        (**self).increment(key, by).await
    }
    async fn decrement(&self, key: &str, by: i64) -> Result<i64> {
        (**self).decrement(key, by).await
    }
    async fn clear(&self) -> Result<()> {
        (**self).clear().await
    }
}
