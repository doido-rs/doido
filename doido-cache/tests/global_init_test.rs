//! `global::init` builds the store from config and is idempotent.
//!
//! Separate test binary from `global_test` so it owns a fresh global `OnceLock`.
//! With no `config/<env>.yml` in the test working directory, `init` falls back to
//! the in-memory backend.

use doido_cache::global;
use serde_json::json;

#[tokio::test]
async fn init_builds_memory_store_and_is_idempotent() {
    let store = global::init().await.unwrap();
    store.set("k", json!("v"), None).await.unwrap();
    assert_eq!(store.get("k").await.unwrap(), Some(json!("v")));

    // A second init returns the same installed store, not a fresh one.
    let again = global::init().await.unwrap();
    assert_eq!(again.get("k").await.unwrap(), Some(json!("v")));
}
