+++
title = "Cache"
description = "Un cache store conectable con TTLs, namespacing, fetch read-through y múltiples stores con nombre."
weight = 10
+++

> **Especificación de diseño:** [`docs/10-cache.md`](https://github.com/doido-rs/doido/blob/master/docs/10-cache.md).
> Esta guía documenta la API tal como está implementada en `doido-cache`.

**Análogo en Rails: Active Support Cache.** Un único trait `CacheStore` abstrae backends en
memoria, Redis, Memcached y base de datos. Las claves pueden llevar namespace, los valores
se serializan en JSON, y un `fetch` read-through calcula en el miss. El mismo store respalda
las [sesiones](@/docs/reference/middleware.es.md), el rate limiting y la
[caché de fragmentos](@/docs/reference/views.es.md).

## Vistazo general

```rust
use doido::cache::{CacheStore, MemoryStore, NamespacedStore, CacheRegistry};
use doido::cache::fetch::fetch;
```

## El trait del store

`CacheStore` es un trait async con `get`, `set` (con TTL opcional), `delete`, `exists`,
`increment`, `decrement` y `clear`. Los valores son `serde_json::Value`.

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

Elige un backend en la sección `cache` de la config: `memory` (por defecto), `redis`
(feature `cache-redis`), `memcache` (feature `cache-memcache`) o `db` (feature `cache-db`).
Construye uno a partir de `CacheConfig`, o constrúyelo directamente.

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

`NamespacedStore` prefija transparentemente cada clave, así que múltiples apps o entornos
pueden compartir un backend sin colisiones.

```rust
use doido::cache::{MemoryStore, NamespacedStore};

let store = NamespacedStore::new(MemoryStore::new(), "myapp:production");
store.set("posts/all", json!([]), None).await?; // almacenado como "myapp:production:posts/all"
```

## Fetch read-through

`fetch` devuelve el valor cacheado para una clave, o ejecuta la closure async para
calcularlo, almacenarlo y devolverlo — la closure se ejecuta solo en un miss.

```rust
use doido::cache::fetch::fetch;

let posts = fetch(&store, "posts/all", Some(60), async || {
    // query costosa, solo en un cache miss
    json!(load_posts().await)
}).await;
```

## Stores con nombre

Registra múltiples stores por nombre en un `CacheRegistry` (p. ej. un store `default` y uno
`sessions`), o instala un valor global por defecto del proceso con `init_cache` y léelo con
`global::store()`.

```rust
use doido::cache::CacheRegistry;
use std::sync::Arc;

let mut registry = CacheRegistry::new();
registry.add("default", Arc::new(MemoryStore::new()));
registry.add("sessions", Arc::new(NamespacedStore::new(MemoryStore::new(), "sess")));

let store = registry.store("default").unwrap();
```

```yaml
# múltiples stores con nombre
cache:
  stores:
    default:  { type: redis, namespace: app }
    sessions: { type: memory, namespace: sess }
```

## Pruebas

`MemoryStore` siempre está disponible y no necesita setup, así que el código que depende de
la caché es determinista en las pruebas — establece, luego verifica con `get`.

```rust
let store = MemoryStore::new();
store.set("k", json!(5), None).await?;
assert_eq!(store.get("k").await?, Some(json!(5)));
```

## Véase también

- [Middleware y sesiones](@/docs/reference/middleware.es.md) — `CacheSessionStore` y rate limiting.
- [Vistas](@/docs/reference/views.es.md) — caché de fragmentos vía `cache_fragment`.
- [Jobs](@/docs/reference/jobs.es.md) — los backends Redis/DB son infraestructura compartida.
