use doido_cache::{CacheRegistry, CacheStore, MemoryStore, NamespacedStore};
use std::sync::Arc;
use serde_json::json;

#[test]
fn test_full_roundtrip_through_named_namespaced_registry() {
    let mut reg = CacheRegistry::new();
    let ns_store = NamespacedStore::new(MemoryStore::new(), "myapp:test");
    reg.add("primary", Arc::new(ns_store));

    let store = reg.store("primary").unwrap();
    store.set("user:1", json!({"name": "Alice"}), None).unwrap();
    let val = store.get("user:1").unwrap().unwrap();
    assert_eq!(val["name"], "Alice");
}

#[test]
fn test_memory_store_as_dyn_cache_store() {
    let store: Arc<dyn CacheStore> = Arc::new(MemoryStore::new());
    store.set("x", json!(99), None).unwrap();
    assert_eq!(store.get("x").unwrap(), Some(json!(99)));
    store.delete("x").unwrap();
    assert!(store.get("x").unwrap().is_none());
}
