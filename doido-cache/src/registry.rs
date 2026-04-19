use crate::store::CacheStore;
use std::{collections::HashMap, sync::Arc};

pub struct CacheRegistry {
    stores: HashMap<String, Arc<dyn CacheStore>>,
}

impl CacheRegistry {
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: impl Into<String>, store: Arc<dyn CacheStore>) {
        self.stores.insert(name.into(), store);
    }

    pub fn store(&self, name: &str) -> Option<Arc<dyn CacheStore>> {
        self.stores.get(name).cloned()
    }
}

impl Default for CacheRegistry {
    fn default() -> Self {
        Self::new()
    }
}
