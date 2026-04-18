use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};
use serde_json::Value;
use crate::store::CacheStore;
use doido_core::Result;

pub struct MemoryStore {
    data: RwLock<HashMap<String, (Value, Option<Instant>)>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self { data: RwLock::new(HashMap::new()) }
    }
}

impl Default for MemoryStore {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl CacheStore for MemoryStore {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        let guard = self.data.read().unwrap();
        match guard.get(key) {
            None => Ok(None),
            Some((val, None)) => Ok(Some(val.clone())),
            Some((val, Some(expiry))) => {
                if Instant::now() > *expiry {
                    drop(guard);
                    self.data.write().unwrap().remove(key);
                    Ok(None)
                } else {
                    Ok(Some(val.clone()))
                }
            }
        }
    }

    async fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()> {
        let expiry = ttl_secs.map(|s| Instant::now() + Duration::from_secs(s));
        self.data.write().unwrap().insert(key.to_string(), (value, expiry));
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.data.write().unwrap().remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }

    async fn increment(&self, key: &str, by: i64) -> Result<i64> {
        let mut data = self.data.write().unwrap();
        let entry = data.entry(key.to_string()).or_insert((serde_json::json!(0), None));
        let current = entry.0.as_i64().unwrap_or(0);
        let new_val = current + by;
        entry.0 = serde_json::json!(new_val);
        Ok(new_val)
    }

    async fn decrement(&self, key: &str, by: i64) -> Result<i64> {
        self.increment(key, -by).await
    }

    async fn clear(&self) -> Result<()> {
        self.data.write().unwrap().clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryStore;
    use crate::store::CacheStore;
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn test_set_and_get() {
        let s = MemoryStore::new();
        s.set("k", json!("v"), None).await.unwrap();
        assert_eq!(s.get("k").await.unwrap(), Some(json!("v")));
    }

    #[tokio::test]
    async fn test_delete() {
        let s = MemoryStore::new();
        s.set("k", json!(1), None).await.unwrap();
        s.delete("k").await.unwrap();
        assert!(s.get("k").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_exists() {
        let s = MemoryStore::new();
        assert!(!s.exists("k").await.unwrap());
        s.set("k", json!(true), None).await.unwrap();
        assert!(s.exists("k").await.unwrap());
    }

    #[tokio::test]
    async fn test_ttl_expires() {
        let s = MemoryStore::new();
        s.set("k", json!("v"), Some(0)).await.unwrap(); // 0s TTL — expires immediately
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(s.get("k").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_increment() {
        let s = MemoryStore::new();
        assert_eq!(s.increment("counter", 1).await.unwrap(), 1);
        assert_eq!(s.increment("counter", 4).await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_decrement() {
        let s = MemoryStore::new();
        s.increment("counter", 10).await.unwrap();
        assert_eq!(s.decrement("counter", 3).await.unwrap(), 7);
    }

    #[tokio::test]
    async fn test_clear() {
        let s = MemoryStore::new();
        s.set("a", json!(1), None).await.unwrap();
        s.set("b", json!(2), None).await.unwrap();
        s.clear().await.unwrap();
        assert!(s.get("a").await.unwrap().is_none());
        assert!(s.get("b").await.unwrap().is_none());
    }
}
