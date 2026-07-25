//! Fragment caching (Rails `<% cache key do %> … <% end %>`).
//!
//! [`cache_fragment`] returns the cached HTML for `key`, computing and storing it
//! only on a miss, against any [`doido_cache::CacheStore`].

use doido_cache::CacheStore;
use std::sync::Arc;

/// Return the cached fragment for `key`, or compute it with `render`, store it,
/// and return it. `render` runs only on a cache miss.
pub async fn cache_fragment(
    store: &Arc<dyn CacheStore>,
    key: &str,
    render: impl FnOnce() -> String,
) -> String {
    if let Ok(Some(value)) = store.get(key).await {
        if let Some(html) = value.as_str() {
            return html.to_string();
        }
    }
    let html = render();
    let _ = store
        .set(key, serde_json::Value::String(html.clone()), None)
        .await;
    html
}
