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

impl CacheStore for MemoryStore {
    fn get(&self, key: &str) -> Result<Option<Value>> {
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

    fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()> {
        let expiry = ttl_secs.map(|s| Instant::now() + Duration::from_secs(s));
        self.data.write().unwrap().insert(key.to_string(), (value, expiry));
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.data.write().unwrap().remove(key);
        Ok(())
    }

    fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryStore;
    use crate::store::CacheStore;
    use serde_json::json;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_set_and_get() {
        let s = MemoryStore::new();
        s.set("k", json!("v"), None).unwrap();
        assert_eq!(s.get("k").unwrap(), Some(json!("v")));
    }

    #[test]
    fn test_delete() {
        let s = MemoryStore::new();
        s.set("k", json!(1), None).unwrap();
        s.delete("k").unwrap();
        assert!(s.get("k").unwrap().is_none());
    }

    #[test]
    fn test_exists() {
        let s = MemoryStore::new();
        assert!(!s.exists("k").unwrap());
        s.set("k", json!(true), None).unwrap();
        assert!(s.exists("k").unwrap());
    }

    #[test]
    fn test_ttl_expires() {
        let s = MemoryStore::new();
        s.set("k", json!("v"), Some(0)).unwrap(); // 0s TTL — expires immediately
        thread::sleep(Duration::from_millis(10));
        assert!(s.get("k").unwrap().is_none());
    }
}
