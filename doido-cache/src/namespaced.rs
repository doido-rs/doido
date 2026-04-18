use crate::store::CacheStore;
use doido_core::Result;
use serde_json::Value;

pub struct NamespacedStore<S: CacheStore> {
    inner: S,
    prefix: String,
}

impl<S: CacheStore> NamespacedStore<S> {
    pub fn new(inner: S, prefix: impl Into<String>) -> Self {
        Self { inner, prefix: prefix.into() }
    }

    fn full_key(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, key)
    }
}

impl<S: CacheStore> CacheStore for NamespacedStore<S> {
    fn get(&self, key: &str) -> Result<Option<Value>> { self.inner.get(&self.full_key(key)) }
    fn set(&self, key: &str, value: Value, ttl: Option<u64>) -> Result<()> { self.inner.set(&self.full_key(key), value, ttl) }
    fn delete(&self, key: &str) -> Result<()> { self.inner.delete(&self.full_key(key)) }
    fn exists(&self, key: &str) -> Result<bool> { self.inner.exists(&self.full_key(key)) }
}

#[cfg(test)]
mod tests {
    use super::NamespacedStore;
    use crate::{memory::MemoryStore, store::CacheStore};
    use serde_json::json;

    #[test]
    fn test_namespaced_store_prepends_prefix() {
        let inner = MemoryStore::new();
        let ns = NamespacedStore::new(inner, "myapp:prod:custom");
        ns.set("users:1", json!("alice"), None).unwrap();
        assert_eq!(ns.get("users:1").unwrap(), Some(json!("alice")));
        assert!(ns.get("non:existent").unwrap().is_none());
    }

    #[test]
    fn test_namespaced_delete() {
        let ns = NamespacedStore::new(MemoryStore::new(), "app");
        ns.set("k", json!(1), None).unwrap();
        ns.delete("k").unwrap();
        assert!(ns.get("k").unwrap().is_none());
    }
}
