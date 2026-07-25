//! Read-through caching (Rails `Rails.cache.fetch(key) { … }`).

use crate::store::CacheStore;
use doido_core::Result;
use serde_json::Value;

/// Return the cached value for `key`, or compute it with `render`, store it
/// (with optional `ttl_secs`), and return it. `render` runs only on a miss.
pub async fn fetch(
    store: &dyn CacheStore,
    key: &str,
    ttl_secs: Option<u64>,
    render: impl AsyncFnOnce() -> Value,
) -> Result<Value> {
    if let Some(value) = store.get(key).await? {
        return Ok(value);
    }
    let value = render().await;
    store.set(key, value.clone(), ttl_secs).await?;
    Ok(value)
}
