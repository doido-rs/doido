//! Recyclable cache versioning (Rails `cache_versioning`): a stored entry
//! carries a version, and a read with a different version is a miss — so bumping
//! the version cheaply invalidates without deleting keys.

use crate::store::CacheStore;
use doido_core::Result;
use serde_json::{json, Value};

/// A versioned key: `versioned_key("post/1", 3)` → `"post/1:v3"`.
pub fn versioned_key(key: &str, version: u64) -> String {
    format!("{key}:v{version}")
}

/// Store `value` under `key` tagged with `version`.
pub async fn write_versioned(
    store: &dyn CacheStore,
    key: &str,
    version: u64,
    value: Value,
    ttl_secs: Option<u64>,
) -> Result<()> {
    store
        .set(key, json!({ "__v": version, "value": value }), ttl_secs)
        .await
}

/// Read `key` only if its stored version matches `version`; a mismatch (or a
/// missing key) is a miss.
pub async fn read_versioned(
    store: &dyn CacheStore,
    key: &str,
    version: u64,
) -> Result<Option<Value>> {
    match store.get(key).await? {
        Some(entry) if entry.get("__v") == Some(&json!(version)) => Ok(entry.get("value").cloned()),
        _ => Ok(None),
    }
}
