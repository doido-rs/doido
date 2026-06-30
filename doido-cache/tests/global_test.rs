//! Tests for the process-global default cache store.
//!
//! The global is a process-wide `OnceLock`, so this binary installs it exactly
//! once — the whole lifecycle lives in a single test.

use doido_cache::{global, CacheStore, MemoryStore};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn set_store_then_global_accessors_return_it() {
    // Nothing installed yet.
    assert!(global::try_store().is_none());

    let store: Arc<dyn CacheStore> = Arc::new(MemoryStore::new());
    global::set_store(store).expect("first install succeeds");

    // Both accessors now see the installed store.
    assert!(global::try_store().is_some());
    let store = global::store();
    store.set("k", json!(5), None).await.unwrap();
    assert_eq!(store.get("k").await.unwrap(), Some(json!(5)));

    // A second install is rejected.
    let second: Arc<dyn CacheStore> = Arc::new(MemoryStore::new());
    assert!(global::set_store(second).is_err());
}
