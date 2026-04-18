use doido_cache::{CacheRegistry, MemoryStore, CacheStore};
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
