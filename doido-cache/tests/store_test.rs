use doido_cache::CacheStore;
use serde_json::Value;

struct FakeStore;

#[async_trait::async_trait]
impl CacheStore for FakeStore {
    async fn get(&self, _k: &str) -> doido_core::Result<Option<Value>> {
        Ok(None)
    }
    async fn set(&self, _k: &str, _v: Value, _t: Option<u64>) -> doido_core::Result<()> {
        Ok(())
    }
    async fn delete(&self, _k: &str) -> doido_core::Result<()> {
        Ok(())
    }
    async fn exists(&self, _k: &str) -> doido_core::Result<bool> {
        Ok(false)
    }
    async fn increment(&self, _k: &str, _by: i64) -> doido_core::Result<i64> {
        Ok(0)
    }
    async fn decrement(&self, _k: &str, _by: i64) -> doido_core::Result<i64> {
        Ok(0)
    }
    async fn clear(&self) -> doido_core::Result<()> {
        Ok(())
    }
}

#[test]
fn test_cache_store_trait_is_object_safe() {
    let _store: &dyn CacheStore = &FakeStore;
}

#[tokio::test]
async fn test_fake_store_get_returns_none() {
    let store: &dyn CacheStore = &FakeStore;
    assert!(store.get("k").await.unwrap().is_none());
}
