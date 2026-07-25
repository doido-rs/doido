//! Batch cache operations (Rails `read_multi`/`write_multi`/`fetch_multi`).

use crate::store::CacheStore;
use doido_core::Result;
use serde_json::Value;
use std::collections::BTreeMap;

/// Read several keys at once; only present keys appear in the result.
pub async fn read_multi(store: &dyn CacheStore, keys: &[&str]) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for &key in keys {
        if let Some(value) = store.get(key).await? {
            out.insert(key.to_string(), value);
        }
    }
    Ok(out)
}

/// Write several key/value pairs at once.
pub async fn write_multi(
    store: &dyn CacheStore,
    entries: &[(&str, Value)],
    ttl_secs: Option<u64>,
) -> Result<()> {
    for (key, value) in entries {
        store.set(key, value.clone(), ttl_secs).await?;
    }
    Ok(())
}

/// Read several keys, computing (and caching) any misses with `render`.
pub async fn fetch_multi(
    store: &dyn CacheStore,
    keys: &[&str],
    ttl_secs: Option<u64>,
    render: impl Fn(&str) -> Value,
) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for &key in keys {
        let value = match store.get(key).await? {
            Some(value) => value,
            None => {
                let computed = render(key);
                store.set(key, computed.clone(), ttl_secs).await?;
                computed
            }
        };
        out.insert(key.to_string(), value);
    }
    Ok(out)
}
