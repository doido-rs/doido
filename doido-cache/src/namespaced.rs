use crate::store::CacheStore;
use doido_core::Result;
use serde_json::Value;

pub struct NamespacedStore<S: CacheStore> {
    inner: S,
    prefix: String,
}

impl<S: CacheStore> NamespacedStore<S> {
    pub fn new(inner: S, prefix: impl Into<String>) -> Self {
        Self {
            inner,
            prefix: prefix.into(),
        }
    }

    fn full_key(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, key)
    }
}

#[async_trait::async_trait]
impl<S: CacheStore + Send + Sync> CacheStore for NamespacedStore<S> {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        self.inner.get(&self.full_key(key)).await
    }
    async fn set(&self, key: &str, value: Value, ttl: Option<u64>) -> Result<()> {
        self.inner.set(&self.full_key(key), value, ttl).await
    }
    async fn delete(&self, key: &str) -> Result<()> {
        self.inner.delete(&self.full_key(key)).await
    }
    async fn exists(&self, key: &str) -> Result<bool> {
        self.inner.exists(&self.full_key(key)).await
    }
    async fn increment(&self, key: &str, by: i64) -> Result<i64> {
        self.inner.increment(&self.full_key(key), by).await
    }
    async fn decrement(&self, key: &str, by: i64) -> Result<i64> {
        self.inner.decrement(&self.full_key(key), by).await
    }
    async fn clear(&self) -> Result<()> {
        self.inner.clear().await
    }
}
