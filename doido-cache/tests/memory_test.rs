use doido_cache::MemoryStore;
use doido_cache::CacheStore;
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn test_set_and_get() {
    let s = MemoryStore::new();
    s.set("k", json!("v"), None).await.unwrap();
    assert_eq!(s.get("k").await.unwrap(), Some(json!("v")));
}

#[tokio::test]
async fn test_delete() {
    let s = MemoryStore::new();
    s.set("k", json!(1), None).await.unwrap();
    s.delete("k").await.unwrap();
    assert!(s.get("k").await.unwrap().is_none());
}

#[tokio::test]
async fn test_exists() {
    let s = MemoryStore::new();
    assert!(!s.exists("k").await.unwrap());
    s.set("k", json!(true), None).await.unwrap();
    assert!(s.exists("k").await.unwrap());
}

#[tokio::test]
async fn test_ttl_expires() {
    let s = MemoryStore::new();
    s.set("k", json!("v"), Some(0)).await.unwrap(); // 0s TTL — expires immediately
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(s.get("k").await.unwrap().is_none());
}

#[tokio::test]
async fn test_increment() {
    let s = MemoryStore::new();
    assert_eq!(s.increment("counter", 1).await.unwrap(), 1);
    assert_eq!(s.increment("counter", 4).await.unwrap(), 5);
}

#[tokio::test]
async fn test_decrement() {
    let s = MemoryStore::new();
    s.increment("counter", 10).await.unwrap();
    assert_eq!(s.decrement("counter", 3).await.unwrap(), 7);
}

#[tokio::test]
async fn test_clear() {
    let s = MemoryStore::new();
    s.set("a", json!(1), None).await.unwrap();
    s.set("b", json!(2), None).await.unwrap();
    s.clear().await.unwrap();
    assert!(s.get("a").await.unwrap().is_none());
    assert!(s.get("b").await.unwrap().is_none());
}
