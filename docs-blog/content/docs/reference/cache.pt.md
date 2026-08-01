+++
title = "Cache"
description = "Um cache store plugável com TTLs, namespacing, fetch read-through e múltiplos stores nomeados."
weight = 10
+++

> **Especificação de design:** [`docs/10-cache.md`](https://github.com/doido-rs/doido/blob/master/docs/10-cache.md).
> Este guia documenta a API como implementada em `doido-cache`.

**Análogo no Rails: Active Support Cache.** Um único trait `CacheStore` abstrai backends em
memória, Redis, Memcached e banco de dados. As chaves podem ter namespace, os valores são
serializados em JSON, e um `fetch` read-through computa no miss. O mesmo store apoia
[sessões](@/docs/reference/middleware.pt.md), rate limiting e o
[cache de fragmento](@/docs/reference/views.pt.md).

## Visão geral

```rust
use doido::cache::{CacheStore, MemoryStore, NamespacedStore, CacheRegistry};
use doido::cache::fetch::fetch;
```

## O trait do store

`CacheStore` é um trait async com `get`, `set` (com TTL opcional), `delete`, `exists`,
`increment`, `decrement` e `clear`. Os valores são `serde_json::Value`.

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

Escolha um backend na seção `cache` da config: `memory` (padrão), `redis` (feature
`cache-redis`), `memcache` (feature `cache-memcache`) ou `db` (feature `cache-db`). Construa
um a partir de `CacheConfig`, ou construa-o diretamente.

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

`NamespacedStore` prefixa transparentemente cada chave, então múltiplas apps ou ambientes
podem compartilhar um backend sem colisões.

```rust
use doido::cache::{MemoryStore, NamespacedStore};

let store = NamespacedStore::new(MemoryStore::new(), "myapp:production");
store.set("posts/all", json!([]), None).await?; // armazenado como "myapp:production:posts/all"
```

## Fetch read-through

`fetch` retorna o valor cacheado para uma chave, ou roda a closure async para computá-lo,
armazená-lo e retorná-lo — a closure roda apenas em um miss.

```rust
use doido::cache::fetch::fetch;

let posts = fetch(&store, "posts/all", Some(60), async || {
    // query cara, apenas em um cache miss
    json!(load_posts().await)
}).await;
```

## Stores nomeados

Registre múltiplos stores por nome em um `CacheRegistry` (ex.: um store `default` e um
`sessions`), ou instale um padrão global do processo com `init_cache` e leia-o de volta com
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
# múltiplos stores nomeados
cache:
  stores:
    default:  { type: redis, namespace: app }
    sessions: { type: memory, namespace: sess }
```

## Testes

`MemoryStore` está sempre disponível e não precisa de setup, então o código que depende de
cache é determinístico em testes — defina, depois verifique com `get`.

```rust
let store = MemoryStore::new();
store.set("k", json!(5), None).await?;
assert_eq!(store.get("k").await?, Some(json!(5)));
```

## Veja também

- [Middleware & sessões](@/docs/reference/middleware.pt.md) — `CacheSessionStore` e rate limiting.
- [Views](@/docs/reference/views.pt.md) — cache de fragmento via `cache_fragment`.
- [Jobs](@/docs/reference/jobs.pt.md) — os backends Redis/DB são infraestrutura compartilhada.
