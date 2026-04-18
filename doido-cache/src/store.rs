use serde_json::Value;
use doido_core::Result;

pub trait CacheStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Value>>;
    fn set(&self, key: &str, value: Value, ttl_secs: Option<u64>) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
    fn exists(&self, key: &str) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::{CacheStore, Value};

    struct FakeStore;
    impl CacheStore for FakeStore {
        fn get(&self, _key: &str) -> doido_core::Result<Option<Value>> { Ok(None) }
        fn set(&self, _key: &str, _v: Value, _ttl: Option<u64>) -> doido_core::Result<()> { Ok(()) }
        fn delete(&self, _key: &str) -> doido_core::Result<()> { Ok(()) }
        fn exists(&self, _key: &str) -> doido_core::Result<bool> { Ok(false) }
    }

    #[test]
    fn test_cache_store_trait_is_object_safe() {
        let store: &dyn CacheStore = &FakeStore;
        assert!(store.get("k").unwrap().is_none());
    }
}
