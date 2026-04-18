use std::{collections::HashMap, sync::Arc};
use crate::store::CacheStore;

pub struct CacheRegistry {
    stores: HashMap<String, Arc<dyn CacheStore>>,
}

impl CacheRegistry {
    pub fn new() -> Self {
        Self { stores: HashMap::new() }
    }

    pub fn add(&mut self, name: impl Into<String>, store: Arc<dyn CacheStore>) {
        self.stores.insert(name.into(), store);
    }

    pub fn store(&self, name: &str) -> Option<Arc<dyn CacheStore>> {
        self.stores.get(name).cloned()
    }
}

impl Default for CacheRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::CacheRegistry;
    use crate::memory::MemoryStore;
    use std::sync::Arc;
    use serde_json::json;

    #[tokio::test]
    async fn test_registry_add_and_retrieve() {
        let mut reg = CacheRegistry::new();
        reg.add("default", Arc::new(MemoryStore::new()));
        let store = reg.store("default").unwrap();
        store.set("k", json!(42), None).await.unwrap();
        assert_eq!(store.get("k").await.unwrap(), Some(json!(42)));
    }

    #[test]
    fn test_registry_missing_store_returns_none() {
        let reg = CacheRegistry::new();
        assert!(reg.store("nonexistent").is_none());
    }
}
