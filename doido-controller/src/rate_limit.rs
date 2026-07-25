//! Fixed-window rate limiting (Rails 8 `rate_limit to:, within:`), backed by any
//! [`doido_cache::CacheStore`].
//!
//! Each key counts requests within a window; the stored entry carries the window
//! start, so the window resets correctly regardless of the cache's TTL support.

use doido_cache::CacheStore;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Allow at most `limit` requests per `window_secs` per key.
pub struct RateLimiter {
    store: Arc<dyn CacheStore>,
    limit: i64,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(store: Arc<dyn CacheStore>, limit: i64, window_secs: u64) -> Self {
        Self {
            store,
            limit,
            window_secs,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Record a hit for `key`, returning `true` if it is within the limit.
    pub async fn check(&self, key: &str) -> bool {
        let full_key = format!("ratelimit:{key}");
        let now = Self::now();

        let entry = self
            .store
            .get(&full_key)
            .await
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_value::<(i64, u64)>(v).ok());

        // Reset the window if it has elapsed (or there is no entry yet).
        let (count, start) = match entry {
            Some((count, start)) if now.saturating_sub(start) < self.window_secs => (count, start),
            _ => (0, now),
        };

        if count >= self.limit {
            return false;
        }

        let value = serde_json::to_value((count + 1, start)).unwrap_or(serde_json::Value::Null);
        let _ = self
            .store
            .set(&full_key, value, Some(self.window_secs))
            .await;
        true
    }
}
