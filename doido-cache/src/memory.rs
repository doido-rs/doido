use crate::store::CacheStore;
use doido_core::Result;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};

pub struct MemoryStore {
    data: RwLock<HashMap<String, (Value, Option<Instant>)>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
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
        self.data
            .write()
            .unwrap()
            .insert(key.to_string(), (value, expiry));
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
        let entry = data
            .entry(key.to_string())
            .or_insert((serde_json::json!(0), None));
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
