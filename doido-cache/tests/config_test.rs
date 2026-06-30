//! Tests for the `cache` config section and the backend factory.

use doido_cache::{CacheBackend, CacheConfig};
use doido_cache::config::YamlConfig;
use serde_json::json;

#[test]
fn defaults_to_memory_backend() {
    let config = CacheConfig::default();
    assert_eq!(config.backend, CacheBackend::Memory);
    assert!(config.endpoint.is_none());
    assert!(config.namespace.is_none());
}

#[test]
fn parses_full_redis_section() {
    let yaml = "cache:\n  type: redis\n  endpoint: redis://cache.internal:6379\n  namespace: shop\n";
    let config = YamlConfig::from_yaml(yaml).unwrap().cache;
    assert_eq!(config.backend, CacheBackend::Redis);
    assert_eq!(config.endpoint.as_deref(), Some("redis://cache.internal:6379"));
    assert_eq!(config.namespace.as_deref(), Some("shop"));
}

#[test]
fn parses_memcache_backend() {
    let config = YamlConfig::from_yaml("cache:\n  type: memcache\n")
        .unwrap()
        .cache;
    assert_eq!(config.backend, CacheBackend::Memcache);
}

#[test]
fn ignores_unrelated_sections_and_defaults_cache() {
    // A config file that has no `cache` section still parses; cache defaults.
    let yaml = "server:\n  bind: 0.0.0.0\n  port: 3000\n";
    let config = YamlConfig::from_yaml(yaml).unwrap();
    assert_eq!(config.cache.backend, CacheBackend::Memory);
}

#[tokio::test]
async fn builds_a_working_memory_store() {
    let store = CacheConfig::default().build().await.unwrap();
    store.set("k", json!("v"), None).await.unwrap();
    assert_eq!(store.get("k").await.unwrap(), Some(json!("v")));
    store.delete("k").await.unwrap();
    assert!(store.get("k").await.unwrap().is_none());
}

#[tokio::test]
async fn builds_namespaced_memory_store() {
    let config = CacheConfig {
        backend: CacheBackend::Memory,
        endpoint: None,
        namespace: Some("ns".to_string()),
    };
    let store = config.build().await.unwrap();
    store.set("user:1", json!(1), None).await.unwrap();
    // Round-trips through the namespacing wrapper transparently.
    assert_eq!(store.get("user:1").await.unwrap(), Some(json!(1)));
}

// When the `cache-redis` feature is off, selecting redis must fail with a clear,
// actionable error rather than silently falling back.
#[cfg(not(feature = "cache-redis"))]
#[tokio::test]
async fn redis_without_feature_errors_clearly() {
    let config = CacheConfig {
        backend: CacheBackend::Redis,
        endpoint: Some("redis://127.0.0.1:6379".to_string()),
        namespace: None,
    };
    let err = config.build().await.unwrap_err().to_string();
    assert!(err.contains("cache-redis"), "unexpected error: {err}");
}

#[cfg(not(feature = "cache-memcache"))]
#[tokio::test]
async fn memcache_without_feature_errors_clearly() {
    let config = CacheConfig {
        backend: CacheBackend::Memcache,
        endpoint: Some("memcache://127.0.0.1:11211".to_string()),
        namespace: None,
    };
    let err = config.build().await.unwrap_err().to_string();
    assert!(err.contains("cache-memcache"), "unexpected error: {err}");
}
