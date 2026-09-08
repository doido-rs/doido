use crate::codec;
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
    compress: bool,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::with_compress(false)
    }

    pub fn with_compress(compress: bool) -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            compress,
        }
    }

    fn stored_value(&self, value: Value) -> Result<Value> {
        if self.compress {
            Ok(Value::String(codec::pack(&value, true)?))
        } else {
            Ok(value)
        }
    }

    fn loaded_value(&self, stored: Value) -> Result<Value> {
        match stored {
            Value::String(raw) if raw.starts_with(codec::GZIP_PREFIX) => codec::unpack(&raw),
            Value::String(raw) if self.compress => codec::unpack(&raw),
            other => Ok(other),
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
            Some((val, None)) => Ok(Some(self.loaded_value(val.clone())?)),
            Some((val, Some(expiry))) => {
                if Instant::now() > *expiry {
                    drop(guard);
                    self.data.write().unwrap().remove(key);
                    Ok(None)
                } else {
                    Ok(Some(self.loaded_value(val.clone())?))
                }
            }
        }
    }

    async fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()> {
        let expiry = ttl_secs.map(|s| Instant::now() + Duration::from_secs(s));
        let stored = self.stored_value(value)?;
        self.data
            .write()
            .unwrap()
            .insert(key.to_string(), (stored, expiry));
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.data.write().unwrap().remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let guard = self.data.read().unwrap();
        match guard.get(key) {
            None => Ok(false),
            Some((_, Some(expiry))) if Instant::now() > *expiry => {
                drop(guard);
                self.data.write().unwrap().remove(key);
                Ok(false)
            }
            Some(_) => Ok(true),
        }
    }

    async fn increment(&self, key: &str, by: i64) -> Result<i64> {
        let mut data = self.data.write().unwrap();
        let entry = data
            .entry(key.to_string())
            .or_insert((serde_json::json!(0), None));
        let current = match &entry.0 {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            Value::String(s) => codec::unpack(s)?.as_i64().unwrap_or(0),
            _ => entry.0.as_i64().unwrap_or(0),
        };
        let new_val = current + by;
        entry.0 = self.stored_value(serde_json::json!(new_val))?;
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
