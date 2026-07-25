//! DB cache store — hermetic (in-memory sqlite). Runs with `--features cache-db`
//! (make test-backends); the feature is off in the fast verify gate.
#![cfg(feature = "cache-db")]

use doido_cache::db_store::DbCacheStore;
use doido_cache::CacheStore;
use doido_model::testing::TestDb;
use serde_json::json;

#[tokio::test]
async fn db_cache_roundtrip_increment_and_clear() {
    let db = TestDb::new().await.unwrap();
    let store = DbCacheStore::connect(db.conn().clone()).await.unwrap();

    store.set("k", json!("v"), None).await.unwrap();
    assert_eq!(store.get("k").await.unwrap(), Some(json!("v")));
    assert!(store.exists("k").await.unwrap());

    store.delete("k").await.unwrap();
    assert!(store.get("k").await.unwrap().is_none());

    assert_eq!(store.increment("n", 5).await.unwrap(), 5);
    assert_eq!(store.increment("n", 3).await.unwrap(), 8);
    assert_eq!(store.decrement("n", 2).await.unwrap(), 6);

    store.clear().await.unwrap();
    assert!(store.get("n").await.unwrap().is_none());
}
