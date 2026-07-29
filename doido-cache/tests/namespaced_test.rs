use doido_cache::{CacheStore, MemoryStore, NamespacedStore};
use serde_json::json;

#[tokio::test]
async fn test_namespaced_store_prepends_prefix() {
    let inner = MemoryStore::new();
    let ns = NamespacedStore::new(inner, "myapp:prod:custom");
    ns.set("users:1", json!("alice"), None).await.unwrap();
    assert_eq!(ns.get("users:1").await.unwrap(), Some(json!("alice")));
    assert!(ns.get("non:existent").await.unwrap().is_none());
}

#[tokio::test]
async fn test_namespaced_delete() {
    let ns = NamespacedStore::new(MemoryStore::new(), "app");
    ns.set("k", json!(1), None).await.unwrap();
    ns.delete("k").await.unwrap();
    assert!(ns.get("k").await.unwrap().is_none());
}

#[tokio::test]
async fn test_namespaced_exists_increment_decrement_and_clear() {
    let inner = MemoryStore::new();
    let ns = NamespacedStore::new(inner, "ns");

    assert!(!ns.exists("counter").await.unwrap());
    ns.set("counter", json!(0), None).await.unwrap();
    assert!(ns.exists("counter").await.unwrap());

    assert_eq!(ns.increment("counter", 5).await.unwrap(), 5);
    assert_eq!(ns.decrement("counter", 2).await.unwrap(), 3);

    ns.set("other", json!("x"), None).await.unwrap();
    ns.clear().await.unwrap();
    assert!(ns.get("counter").await.unwrap().is_none());
    assert!(ns.get("other").await.unwrap().is_none());
}
