# doido-cache — Spec

Rails analogue: **Active Support Cache Store**

## Decisions (resolved in interview)

- **Backends: pluggable** — in-memory, Redis, and Database (sea-orm) selectable via config
- **Namespacing: configurable** — app name + environment prefix by default, fully customizable

## `Cache` Trait (pluggable)

```rust
pub trait CacheStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn increment(&self, key: &str, by: i64) -> Result<i64>;
    async fn decrement(&self, key: &str, by: i64) -> Result<i64>;
    async fn clear(&self) -> Result<()>;
}
```

Higher-level API (built on top of the trait, available to all backends):

```rust
// Read or populate on miss
cache.fetch("posts/all", ttl, || async { Post::find().all(&db).await }).await?;

// Typed read/write (serde JSON serialization)
cache.read::<Vec<Post>>("posts/all").await?
cache.write("posts/all", &posts, Some(Duration::from_secs(300))).await?;
```

## Built-in Backends

| Backend | Feature flag | Description |
|---------|-------------|-------------|
| `MemoryStore` | default | `HashMap` + TTL, single-process, dev/test |
| `RedisStore` | `feature = "cache-redis"` | Redis `GET`/`SET EX`, distributed |
| `MemcacheStore` | `feature = "cache-memcache"` | Memcached, distributed |

Selected via the `cache` section of `config/<env>.yml`:

```yaml
cache:
  type: redis                       # memory (default) | redis | memcache
  endpoint: redis://127.0.0.1:6379  # backend address (redis/memcache); memory ignores it
  namespace: myapp                  # optional key prefix (via NamespacedStore)
```

`endpoint` can be overridden per environment with the `CACHE__ENDPOINT` env var
(the same `SECTION__KEY` convention used elsewhere). The Redis and Memcached
backends compile only when the matching feature is enabled on `doido-cache`
(forwarded by the `doido` facade as `cache-redis` / `cache-memcache`); selecting
a backend whose feature is off returns a clear error at boot.

`doido_cache::config::CacheConfig::build()` turns the config into an
`Arc<dyn CacheStore>`, and `doido_cache::global::init()` installs it as the
process-global default (read back with `doido_cache::global::store()`), mirroring
the model connection pool.

## Key Namespacing

Final key format: `<app>:<env>:<custom>:<user_key>`

Examples:
```
myapp:production:posts/all
myapp:development:users/42/profile
myapp:test:sessions/abc123
```

Namespace config options:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `namespace.app` | `String` | app name from config | First prefix segment |
| `namespace.env` | `bool` | `true` | Include environment in key |
| `namespace.custom` | `String` | `""` | Extra segment (e.g. service name) |

Namespace applied transparently — user keys are always short; the store handles prefixing internally.

## Multiple Named Stores

Apps can register multiple named cache stores for different purposes:

```rust
// Access a specific named store
let cache = doido_cache::store("sessions")?;
let default = doido_cache::store("default")?;
```

Config:
```toml
[cache.stores.default]
backend = "redis"

[cache.stores.sessions]
backend = "memory"
ttl = 3600

[cache.stores.sessions.namespace]
custom = "sess"
```

## Test Helper

In test environment, `MemoryStore` is always used.  
Test helper resets the store between tests:

```rust
use doido_cache::testing;

testing::reset();                         // clear all entries
testing::assert_cached("posts/all");      // assert key exists
testing::assert_not_cached("posts/all");  // assert key absent
```

## Known Requirements

- `CacheStore` trait is the only abstraction; no concrete type leaks to callers
- Typed `read::<T>` / `write` use `serde_json` for serialization
- `fetch` is atomic on in-memory; best-effort on Redis/DB (no distributed lock in v1)
- Namespacing applied by the `NamespacedStore` wrapper that delegates to any `CacheStore`
- Multiple named stores supported via registry
- In test env, `MemoryStore` always active regardless of config

## TDD Surface

- Test `MemoryStore`: set → get returns value; miss returns `None`
- Test TTL expiry removes entry after duration
- Test `delete` removes key
- Test `fetch` populates on miss, returns cached on subsequent call
- Test `increment` / `decrement` on non-existent key initializes to 0
- Test `clear` removes all entries
- Test namespace prefixing produces correct final key
- Test `env = false` omits environment segment
- Test custom prefix adds segment correctly
- Test typed `read::<T>` / `write` round-trip with serde
- Test multiple named stores are isolated
- Test `testing::reset()` clears between test cases
- Integration test: controller uses cache, `testing::assert_cached` confirms hit
