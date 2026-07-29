use doido_cache::{CacheRegistry, MemoryStore};
use serde_json::json;
use std::sync::Arc;

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

#[test]
fn test_registry_default_is_empty() {
    let reg = CacheRegistry::default();
    assert!(reg.store("any").is_none());
}

#[tokio::test]
async fn test_registry_holds_multiple_named_stores() {
    let mut reg = CacheRegistry::new();
    reg.add("cache", Arc::new(MemoryStore::new()));
    reg.add("sessions", Arc::new(MemoryStore::new()));

    reg.store("cache")
        .unwrap()
        .set("k", json!(1), None)
        .await
        .unwrap();
    reg.store("sessions")
        .unwrap()
        .set("k", json!(2), None)
        .await
        .unwrap();

    assert_eq!(
        reg.store("cache").unwrap().get("k").await.unwrap(),
        Some(json!(1))
    );
    assert_eq!(
        reg.store("sessions").unwrap().get("k").await.unwrap(),
        Some(json!(2))
    );
}
