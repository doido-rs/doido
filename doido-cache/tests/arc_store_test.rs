//! The blanket `CacheStore for Arc<dyn CacheStore>` impl lets a type-erased
//! backend be namespaced and otherwise treated as a `CacheStore`.

use doido_cache::{CacheStore, MemoryStore, NamespacedStore};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn arc_dyn_store_delegates_all_ops() {
    let store: Arc<dyn CacheStore> = Arc::new(MemoryStore::new());

    store.set("n", json!(1), None).await.unwrap();
    assert!(store.exists("n").await.unwrap());
    assert_eq!(store.increment("n", 4).await.unwrap(), 5);
    assert_eq!(store.decrement("n", 2).await.unwrap(), 3);
    store.delete("n").await.unwrap();
    assert!(!store.exists("n").await.unwrap());

    store.set("a", json!("x"), None).await.unwrap();
    store.clear().await.unwrap();
    assert!(store.get("a").await.unwrap().is_none());
}

#[tokio::test]
async fn namespaced_store_wraps_a_boxed_backend() {
    // Wrapping `Arc<dyn CacheStore>` requires the blanket impl.
    let backend: Arc<dyn CacheStore> = Arc::new(MemoryStore::new());
    let namespaced = NamespacedStore::new(backend, "tenant42");

    namespaced.set("profile", json!({"id": 42}), None).await.unwrap();
    let value = namespaced.get("profile").await.unwrap().unwrap();
    assert_eq!(value["id"], 42);
}
