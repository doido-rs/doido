+++
title = "Cache"
description = "A pluggable cache store with TTLs, namespacing, read-through fetch, and multiple named stores."
weight = 10
aliases = ['/docs/guides/cache/']

+++

> **Design spec:** [`docs/10-cache.md`](https://github.com/doido-rs/doido/blob/master/docs/10-cache.md).
> This guide documents the API as implemented in `doido-cache`.

**Rails analogue: Active Support Cache.** A single `CacheStore` trait abstracts over
in-memory, Redis, Memcached, and database backends. Keys can be namespaced, values are
JSON-serialized, and a read-through `fetch` computes-on-miss. The same store backs
[sessions](@/docs/reference/middleware.md), rate limiting, and [fragment caching](@/docs/reference/views.md).

## At a glance

```rust
use doido::cache::{CacheStore, MemoryStore, NamespacedStore, CacheRegistry};
use doido::cache::fetch::fetch;
```

## The store trait

`CacheStore` is an async trait with `get`, `set` (with optional TTL), `delete`, `exists`,
`increment`, `decrement`, and `clear`. Values are `serde_json::Value`.

```rust
use doido::cache::{CacheStore, MemoryStore};
use serde_json::json;

let store = MemoryStore::new();

store.set("user:1", json!({ "name": "Alice" }), Some(300)).await?; // TTL 5 min
let value = store.get("user:1").await?;      // Option<Value>
let hits = store.increment("page:views", 1).await?;
store.delete("user:1").await?;
```

## Backends

Pick a backend in the `cache` config section: `memory` (default), `redis` (feature
`cache-redis`), `memcache` (feature `cache-memcache`), or `db` (feature `cache-db`). Build
one from `CacheConfig`, or construct it directly.

```yaml
# config/production.yml
cache:
  type: redis            # memory | redis | memcache | db
  endpoint: redis://127.0.0.1:6379
  namespace: myapp
```

```rust
use doido::cache::CacheConfig;

let store = CacheConfig::default().build(); // Arc<dyn CacheStore>
```

## Namespacing

`NamespacedStore` transparently prefixes every key, so multiple apps or environments can
share a backend without collisions.

```rust
use doido::cache::{MemoryStore, NamespacedStore};

let store = NamespacedStore::new(MemoryStore::new(), "myapp:production");
store.set("posts/all", json!([]), None).await?; // stored as "myapp:production:posts/all"
```

## Read-through fetch

`fetch` returns the cached value for a key, or runs the async closure to compute it, stores
it, and returns it — the closure runs only on a miss.

```rust
use doido::cache::fetch::fetch;

let posts = fetch(&store, "posts/all", Some(60), async || {
    // expensive query, only on a cache miss
    json!(load_posts().await)
}).await;
```

## Named stores

Register multiple stores by name in a `CacheRegistry` (e.g. a `default` store and a
`sessions` store), or install a process-global default with `init_cache` and read it back
with `global::store()`.

```rust
use doido::cache::CacheRegistry;
use std::sync::Arc;

let mut registry = CacheRegistry::new();
registry.add("default", Arc::new(MemoryStore::new()));
registry.add("sessions", Arc::new(NamespacedStore::new(MemoryStore::new(), "sess")));

let store = registry.store("default").unwrap();
```

```yaml
# multiple named stores
cache:
  stores:
    default:  { type: redis, namespace: app }
    sessions: { type: memory, namespace: sess }
```

## Testing

`MemoryStore` is always available and needs no setup, so cache-dependent code is
deterministic in tests — set, then assert on `get`.

```rust
let store = MemoryStore::new();
store.set("k", json!(5), None).await?;
assert_eq!(store.get("k").await?, Some(json!(5)));
```

## See also

- [Middleware & sessions](@/docs/reference/middleware.md) — `CacheSessionStore` and rate limiting.
- [Views](@/docs/reference/views.md) — fragment caching via `cache_fragment`.
- [Jobs](@/docs/reference/jobs.md) — the Redis/DB backends are shared infrastructure.
