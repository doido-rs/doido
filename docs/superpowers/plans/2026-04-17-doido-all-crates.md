# Doido Framework — Full Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement all remaining Doido crates so the framework is feature-complete according to the spec docs in `docs/`.

**Already done:** `doido-core`, `doido-config`, `doido-router`, `doido-router-macros`, `doido-controller`, `doido-controller-macros`, `doido-view`

**Remaining crates (in dependency order):**

| # | Crate | Depends on | Spec |
|---|-------|-----------|------|
| 1 | `doido-model` | doido-core | docs/03-model.md |
| 2 | `doido-middleware` | doido-core, doido-config | docs/07-middleware.md |
| 3 | `doido-cache` | doido-core, doido-config | docs/10-cache.md |
| 4 | `doido-jobs` | doido-core, doido-config, doido-model | docs/09-jobs.md |
| 5 | `doido-mailer` | doido-core, doido-view, doido-jobs | docs/08-mailer.md |
| 6 | `doido-cable` | doido-core, doido-config | docs/12-cable.md |
| 7 | `doido-kafka` | doido-core, doido-jobs | docs/13-kafka.md |
| 8 | `doido-mcp` | doido-core, doido-config | docs/14-mcp.md |
| 9 | `doido-generators` | doido-core | docs/06b-generators.md |
| 10 | `doido-cli` | all crates | docs/06-cli.md |

Each crate section below follows strict TDD: write failing tests → implement → verify green → commit.

---

## Crate 1: `doido-model`

**Spec:** docs/03-model.md  
**Goal:** Re-export sea-orm fully; add `setup(config)` for the connection pool; expose `doido_model::testing` with an in-memory SQLite helper.

### File Structure

| File | Purpose |
|------|---------|
| `doido-model/Cargo.toml` | Manifest |
| `doido-model/src/lib.rs` | Re-exports + module declarations |
| `doido-model/src/pool.rs` | `setup(config)` → `Arc<DatabaseConnection>` |
| `doido-model/src/testing.rs` | `TestDb::new()` → in-memory SQLite |
| `doido-model/tests/model_test.rs` | Integration tests |

### Task 1.1 — Scaffold

- [ ] **Step 1:** Create `doido-model/Cargo.toml`

```toml
[package]
name = "doido-model"
version = "0.1.0"
edition = "2021"

[dependencies]
doido-core = { path = "../doido-core" }
sea-orm = { version = "1", features = ["sqlx-sqlite", "sqlx-postgres", "runtime-tokio-rustls", "macros"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

[dev-dependencies]
doido-core = { path = "../doido-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2:** Add `doido-model` to workspace `Cargo.toml` members list.

- [ ] **Step 3:** Create `doido-model/src/lib.rs`

```rust
pub use sea_orm;
pub use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    ModelTrait, QueryFilter, Set,
};

pub mod pool;
pub mod testing;
```

- [ ] **Step 4:** Create stub files `pool.rs` and `testing.rs` (empty `// filled in next tasks`).

- [ ] **Step 5:** Run `cargo check -p doido-model` — confirm crate is found.

- [ ] **Step 6:** Commit `feat(model): add doido-model crate scaffold`

### Task 1.2 — `testing` module (TDD)

- [ ] **Step 1:** Write failing test in `doido-model/src/testing.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::TestDb;

    #[tokio::test]
    async fn test_testdb_connects_to_sqlite_in_memory() {
        let db = TestDb::new().await.unwrap();
        let conn = db.conn();
        // confirm connection is alive
        sea_orm::ConnectionTrait::ping(conn).await.unwrap();
    }
}
```

- [ ] **Step 2:** Run `cargo test -p doido-model` — expect compile error (`TestDb` not defined).

- [ ] **Step 3:** Implement `doido-model/src/testing.rs`:

```rust
use sea_orm::{Database, DatabaseConnection};
use doido_core::Result;

pub struct TestDb {
    conn: DatabaseConnection,
}

impl TestDb {
    pub async fn new() -> Result<Self> {
        let conn = Database::connect("sqlite::memory:").await
            .map_err(|e| doido_core::anyhow::anyhow!("TestDb connect failed: {e}"))?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::TestDb;

    #[tokio::test]
    async fn test_testdb_connects_to_sqlite_in_memory() {
        let db = TestDb::new().await.unwrap();
        sea_orm::ConnectionTrait::ping(db.conn()).await.unwrap();
    }
}
```

- [ ] **Step 4:** Run `cargo test -p doido-model` — expect PASS.

- [ ] **Step 5:** Commit `feat(model): add TestDb in-memory SQLite helper`

### Task 1.3 — Integration test

- [ ] **Step 1:** Create `doido-model/tests/model_test.rs`:

```rust
use doido_model::testing::TestDb;

#[tokio::test]
async fn test_sea_orm_re_exports_are_accessible() {
    // Verify re-exports compile and connect
    let db = TestDb::new().await.unwrap();
    doido_model::sea_orm::ConnectionTrait::ping(db.conn()).await.unwrap();
}
```

- [ ] **Step 2:** Run `cargo test -p doido-model --test model_test` — PASS.

- [ ] **Step 3:** Commit `test(model): integration test for sea-orm re-exports and TestDb`

---

## Crate 2: `doido-middleware`

**Spec:** docs/07-middleware.md  
**Goal:** Tower middleware stack — logging + panic recovery always on; CORS, session (cookie-based default) opt-in. Pluggable `SessionStore` trait.

### File Structure

| File | Purpose |
|------|---------|
| `doido-middleware/Cargo.toml` | Manifest |
| `doido-middleware/src/lib.rs` | Module declarations + re-exports |
| `doido-middleware/src/logging.rs` | Request logging middleware |
| `doido-middleware/src/panic.rs` | Panic recovery middleware |
| `doido-middleware/src/session.rs` | `SessionStore` trait + `CookieSessionStore` |
| `doido-middleware/src/cors.rs` | CORS middleware builder |
| `doido-middleware/src/stack.rs` | `MiddlewareStack` builder |
| `doido-middleware/tests/middleware_test.rs` | Integration tests |

### Task 2.1 — Scaffold + `SessionStore` trait (TDD)

- [ ] **Step 1:** Create `doido-middleware/Cargo.toml`:

```toml
[package]
name = "doido-middleware"
version = "0.1.0"
edition = "2021"

[dependencies]
doido-core = { path = "../doido-core" }
axum = { version = "0.8", features = ["macros"] }
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace", "catch-panic"] }
tracing = "0.1"
cookie = { version = "0.18", features = ["secure"] }
serde_json = "1"

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
axum = { version = "0.8", features = ["macros"] }
tower = { version = "0.5", features = ["full"] }
http-body-util = "0.1"
```

- [ ] **Step 2:** Add `doido-middleware` to workspace.

- [ ] **Step 3:** Create all stub source files.

- [ ] **Step 4:** Write failing test for `SessionStore` trait in `session.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{Session, SessionStore};

    struct FakeStore;
    impl SessionStore for FakeStore {
        fn load(&self, _id: &str) -> doido_core::Result<Option<Session>> { Ok(None) }
        fn save(&self, session: &Session) -> doido_core::Result<()> { let _ = session; Ok(()) }
        fn destroy(&self, _id: &str) -> doido_core::Result<()> { Ok(()) }
    }

    #[test]
    fn test_session_store_trait_is_object_safe() {
        let store: &dyn SessionStore = &FakeStore;
        assert!(store.load("x").unwrap().is_none());
    }
}
```

- [ ] **Step 5:** Implement `session.rs`:

```rust
use serde_json::Value;
use doido_core::Result;

#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub data: Value,
}

pub trait SessionStore: Send + Sync {
    fn load(&self, id: &str) -> Result<Option<Session>>;
    fn save(&self, session: &Session) -> Result<()>;
    fn destroy(&self, id: &str) -> Result<()>;
}

pub struct CookieSessionStore;
impl SessionStore for CookieSessionStore {
    fn load(&self, _id: &str) -> Result<Option<Session>> { Ok(None) }
    fn save(&self, _session: &Session) -> Result<()> { Ok(()) }
    fn destroy(&self, _id: &str) -> Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests { /* as above */ }
```

- [ ] **Step 6:** Run `cargo test -p doido-middleware` — PASS.

- [ ] **Step 7:** Commit `feat(middleware): add SessionStore trait and CookieSessionStore`

### Task 2.2 — `MiddlewareStack` builder (TDD)

- [ ] **Step 1:** Write failing test in `stack.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::MiddlewareStack;
    use axum::Router;

    #[test]
    fn test_middleware_stack_builds_without_panic() {
        let app: Router = Router::new();
        let _layered = MiddlewareStack::new().apply(app);
    }

    #[test]
    fn test_middleware_stack_with_cors() {
        let app: Router = Router::new();
        let _layered = MiddlewareStack::new().with_cors().apply(app);
    }
}
```

- [ ] **Step 2:** Implement `stack.rs` using `tower_http` layers.

- [ ] **Step 3:** Run `cargo test -p doido-middleware` — PASS.

- [ ] **Step 4:** Commit `feat(middleware): add MiddlewareStack builder`

### Task 2.3 — Integration test

- [ ] **Step 1:** Create `doido-middleware/tests/middleware_test.rs` verifying a request through the stack returns a valid response via `tower::ServiceExt::oneshot`.

- [ ] **Step 2:** Run `cargo test -p doido-middleware --test middleware_test` — PASS.

- [ ] **Step 3:** Commit `test(middleware): integration test for full middleware stack`

---

## Crate 3: `doido-cache`

**Spec:** docs/10-cache.md  
**Goal:** Pluggable `CacheStore` trait; in-memory backend (default); namespaced key wrapper; named store registry; TTL support.

### File Structure

| File | Purpose |
|------|---------|
| `doido-cache/Cargo.toml` | Manifest |
| `doido-cache/src/lib.rs` | Module declarations + re-exports |
| `doido-cache/src/store.rs` | `CacheStore` trait |
| `doido-cache/src/memory.rs` | `MemoryStore` (default) |
| `doido-cache/src/namespaced.rs` | `NamespacedStore` wrapper |
| `doido-cache/src/registry.rs` | Named store registry |
| `doido-cache/tests/cache_test.rs` | Integration tests |

### Task 3.1 — `CacheStore` trait (TDD)

- [ ] **Step 1:** Create `doido-cache/Cargo.toml`:

```toml
[package]
name = "doido-cache"
version = "0.1.0"
edition = "2021"

[dependencies]
doido-core = { path = "../doido-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["sync", "time"] }

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2:** Add `doido-cache` to workspace.

- [ ] **Step 3:** Write failing test for `CacheStore` trait in `store.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::CacheStore;

    struct FakeStore;
    impl CacheStore for FakeStore {
        fn get(&self, _key: &str) -> doido_core::Result<Option<serde_json::Value>> { Ok(None) }
        fn set(&self, _key: &str, _value: serde_json::Value, _ttl_secs: Option<u64>) -> doido_core::Result<()> { Ok(()) }
        fn delete(&self, _key: &str) -> doido_core::Result<()> { Ok(()) }
        fn exists(&self, _key: &str) -> doido_core::Result<bool> { Ok(false) }
    }

    #[test]
    fn test_cache_store_trait_is_object_safe() {
        let store: &dyn CacheStore = &FakeStore;
        assert!(store.get("k").unwrap().is_none());
    }
}
```

- [ ] **Step 4:** Implement `CacheStore` trait in `store.rs`.

- [ ] **Step 5:** Run `cargo test -p doido-cache` — PASS.

- [ ] **Step 6:** Commit `feat(cache): add CacheStore trait`

### Task 3.2 — `MemoryStore` (TDD)

- [ ] **Step 1:** Write failing tests in `memory.rs`:

```rust
#[test]
fn test_memory_store_set_and_get() { /* set "k" -> "v", get -> Some("v") */ }

#[test]
fn test_memory_store_delete() { /* set "k", delete "k", get -> None */ }

#[test]
fn test_memory_store_ttl_expires() { /* set with ttl=1ms, sleep, get -> None */ }
```

- [ ] **Step 2:** Implement `MemoryStore` with `DashMap` (or `RwLock<HashMap>`) + TTL timestamp check.

- [ ] **Step 3:** Run `cargo test -p doido-cache` — PASS.

- [ ] **Step 4:** Commit `feat(cache): add MemoryStore with TTL support`

### Task 3.3 — `NamespacedStore` + registry (TDD)

- [ ] **Step 1:** Write failing tests in `namespaced.rs`:

```rust
#[test]
fn test_namespaced_store_prepends_prefix() { /* prefix "myapp:prod:custom", key "users:1" -> full key "myapp:prod:custom:users:1" */ }
```

- [ ] **Step 2:** Implement `NamespacedStore<S: CacheStore>` that wraps any store and prepends the namespace.

- [ ] **Step 3:** Implement `CacheRegistry` in `registry.rs` — `add(name, store)` + `store(name) -> &dyn CacheStore`.

- [ ] **Step 4:** Run `cargo test -p doido-cache` — PASS.

- [ ] **Step 5:** Commit `feat(cache): add NamespacedStore wrapper and CacheRegistry`

### Task 3.4 — Integration test

- [ ] **Step 1:** Create `doido-cache/tests/cache_test.rs` covering full roundtrip through named registry with namespace.

- [ ] **Step 2:** Run — PASS.

- [ ] **Step 3:** Commit `test(cache): integration tests for cache registry and namespacing`

---

## Crate 4: `doido-jobs`

**Spec:** docs/09-jobs.md  
**Goal:** Pluggable `JobQueue` trait; `#[job]` macro (max_retries, queue name); in-memory backend; exponential backoff retry; dead-letter queue; worker loop.

### File Structure

| File | Purpose |
|------|---------|
| `doido-jobs/Cargo.toml` | Manifest |
| `doido-jobs-macros/Cargo.toml` | Proc-macro crate |
| `doido-jobs-macros/src/lib.rs` | `#[job]` proc-macro |
| `doido-jobs/src/lib.rs` | Module declarations + re-exports |
| `doido-jobs/src/queue.rs` | `JobQueue` trait + `JobPayload` |
| `doido-jobs/src/memory.rs` | `MemoryQueue` backend |
| `doido-jobs/src/worker.rs` | `Worker` — dequeue + perform loop |
| `doido-jobs/src/retry.rs` | Exponential backoff + dead letter |
| `doido-jobs/tests/jobs_test.rs` | Integration tests |

### Task 4.1 — Scaffold + `JobQueue` trait (TDD)

- [ ] **Step 1:** Create `doido-jobs/Cargo.toml`:

```toml
[package]
name = "doido-jobs"
version = "0.1.0"
edition = "2021"

[dependencies]
doido-core = { path = "../doido-core" }
doido-jobs-macros = { path = "../doido-jobs-macros" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
async-trait = "0.1"
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2:** Create `doido-jobs-macros/Cargo.toml` (proc-macro = true).

- [ ] **Step 3:** Add both to workspace.

- [ ] **Step 4:** Write failing test for `JobQueue` trait.

- [ ] **Step 5:** Implement `JobPayload` struct (id, queue, payload json, attempts, status) and `JobQueue` trait:
  - `enqueue(payload: JobPayload) -> Result<()>`
  - `dequeue(queue: &str) -> Result<Option<JobPayload>>`
  - `ack(id: &str) -> Result<()>`
  - `nack(id: &str, error: &str) -> Result<()>`

- [ ] **Step 6:** Run `cargo test -p doido-jobs` — PASS.

- [ ] **Step 7:** Commit `feat(jobs): add JobQueue trait and JobPayload`

### Task 4.2 — `MemoryQueue` (TDD)

- [ ] **Step 1:** Write failing tests: enqueue → dequeue → ack flow.

- [ ] **Step 2:** Implement `MemoryQueue` with `tokio::sync::Mutex<VecDeque<JobPayload>>`.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(jobs): add MemoryQueue backend`

### Task 4.3 — Retry + dead-letter (TDD)

- [ ] **Step 1:** Write failing tests: nack increments attempts; after max_retries moves to dead-letter queue.

- [ ] **Step 2:** Implement `RetryPolicy` and dead-letter logic in `retry.rs`.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(jobs): add exponential backoff retry and dead-letter queue`

### Task 4.4 — `#[job]` macro (TDD)

- [ ] **Step 1:** Write failing test:

```rust
#[job(max_retries = 3, queue = "default")]
async fn send_welcome_email(user_id: u64) -> doido_core::Result<()> {
    Ok(())
}
// expect: send_welcome_email::enqueue(queue, user_id=1) compiles and enqueues
```

- [ ] **Step 2:** Implement `#[job]` proc-macro generating `enqueue` associated method.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(jobs): add #[job] proc-macro with max_retries and queue`

### Task 4.5 — Integration test

- [ ] **Step 1:** Create `doido-jobs/tests/jobs_test.rs` covering full enqueue → worker dequeue → perform → ack cycle.

- [ ] **Step 2:** Run — PASS.

- [ ] **Step 3:** Commit `test(jobs): integration test for job lifecycle`

---

## Crate 5: `doido-mailer`

**Spec:** docs/08-mailer.md  
**Goal:** `#[mailer]` macro; `Mail` type; pluggable `Deliverer` trait; `deliver_now()` / `deliver_later()` (enqueues `MailDeliveryJob`); `TestDeliverer` for tests; Tera templates via doido-view.

### File Structure

| File | Purpose |
|------|---------|
| `doido-mailer/Cargo.toml` | Manifest |
| `doido-mailer-macros/Cargo.toml` | Proc-macro crate |
| `doido-mailer-macros/src/lib.rs` | `#[mailer]` proc-macro |
| `doido-mailer/src/lib.rs` | Module declarations |
| `doido-mailer/src/mail.rs` | `Mail` struct |
| `doido-mailer/src/deliverer.rs` | `Deliverer` trait + `LogDeliverer` + `TestDeliverer` |
| `doido-mailer/tests/mailer_test.rs` | Integration tests |

### Task 5.1 — `Mail` + `Deliverer` trait (TDD)

- [ ] **Step 1:** Create `doido-mailer/Cargo.toml`:

```toml
[package]
name = "doido-mailer"
version = "0.1.0"
edition = "2021"

[dependencies]
doido-core = { path = "../doido-core" }
doido-view = { path = "../doido-view" }
doido-jobs = { path = "../doido-jobs" }
doido-mailer-macros = { path = "../doido-mailer-macros" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2:** Add both crates to workspace.

- [ ] **Step 3:** Write failing test for `Mail` struct and `Deliverer` trait.

- [ ] **Step 4:** Implement `Mail` (from, to, subject, body_html, body_text) and `Deliverer` trait with `deliver(mail: &Mail) -> Result<()>`.

- [ ] **Step 5:** Implement `LogDeliverer` (logs via tracing) and `TestDeliverer` (captures to `Arc<Mutex<Vec<Mail>>>`).

- [ ] **Step 6:** Run — PASS.

- [ ] **Step 7:** Commit `feat(mailer): add Mail, Deliverer trait, LogDeliverer, TestDeliverer`

### Task 5.2 — `deliver_now` / `deliver_later` + `#[mailer]` macro (TDD)

- [ ] **Step 1:** Write failing test:

```rust
#[mailer]
struct WelcomeMailer;
impl WelcomeMailer {
    fn welcome(user_email: &str) -> Mail {
        Mail::new().to(user_email).subject("Welcome!").body_text("Hi")
    }
}
// WelcomeMailer::welcome("a@b.com").deliver_now(&deliverer).unwrap();
```

- [ ] **Step 2:** Implement `#[mailer]` macro that adds `deliver_now` / `deliver_later` to each method.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(mailer): add #[mailer] macro with deliver_now/deliver_later`

### Task 5.3 — Integration test

- [ ] **Step 1:** Create `doido-mailer/tests/mailer_test.rs` using `TestDeliverer` to assert captured mail.

- [ ] **Step 2:** Run — PASS.

- [ ] **Step 3:** Commit `test(mailer): integration test for mailer delivery pipeline`

---

## Crate 6: `doido-cable`

**Spec:** docs/12-cable.md  
**Goal:** `#[channel]` macro + `Channel` trait; pluggable `PubSub` trait; in-memory backend; ActionCable wire protocol over WebSocket; `Cable::broadcast_to()`; `#[cable_connection]` for identity.

### File Structure

| File | Purpose |
|------|---------|
| `doido-cable/Cargo.toml` | Manifest |
| `doido-cable-macros/Cargo.toml` | Proc-macro crate |
| `doido-cable-macros/src/lib.rs` | `#[channel]` + `#[cable_connection]` macros |
| `doido-cable/src/lib.rs` | Module declarations |
| `doido-cable/src/channel.rs` | `Channel` trait + `ChannelContext` |
| `doido-cable/src/pubsub.rs` | `PubSub` trait + `MemoryPubSub` |
| `doido-cable/src/protocol.rs` | ActionCable wire protocol (JSON) |
| `doido-cable/src/cable.rs` | `Cable` global handle + `broadcast_to` |
| `doido-cable/tests/cable_test.rs` | Integration tests |

### Task 6.1 — `PubSub` trait + `MemoryPubSub` (TDD)

- [ ] **Step 1:** Create `doido-cable/Cargo.toml` with tokio, tokio-tungstenite, serde_json, async-trait.

- [ ] **Step 2:** Write failing tests for `PubSub`: subscribe → publish → receive.

- [ ] **Step 3:** Implement `PubSub` trait + `MemoryPubSub` using `tokio::sync::broadcast`.

- [ ] **Step 4:** Run — PASS.

- [ ] **Step 5:** Commit `feat(cable): add PubSub trait and MemoryPubSub`

### Task 6.2 — `Channel` trait + ActionCable protocol (TDD)

- [ ] **Step 1:** Write failing test for ActionCable JSON message deserialization.

- [ ] **Step 2:** Implement `protocol.rs` (subscribe/unsubscribe/message frame parsing).

- [ ] **Step 3:** Implement `Channel` trait with `subscribed`, `unsubscribed`, `received` async methods.

- [ ] **Step 4:** Run — PASS.

- [ ] **Step 5:** Commit `feat(cable): add Channel trait and ActionCable protocol`

### Task 6.3 — `#[channel]` macro + integration test (TDD)

- [ ] **Step 1:** Write failing test using `#[channel]` macro.

- [ ] **Step 2:** Implement `#[channel]` proc-macro.

- [ ] **Step 3:** Create integration test covering subscribe → publish → receive full flow.

- [ ] **Step 4:** Run — PASS.

- [ ] **Step 5:** Commit `feat(cable): add #[channel] macro and integration tests`

---

## Crate 7: `doido-kafka`

**Spec:** docs/13-kafka.md  
**Goal:** `#[consumer]` macro; `#[topic]` on handler methods; pluggable `MessageCodec` trait; dispatch to doido-jobs after decode; offset commit after successful enqueue.

### File Structure

| File | Purpose |
|------|---------|
| `doido-kafka/Cargo.toml` | Manifest (optional crate) |
| `doido-kafka-macros/Cargo.toml` | Proc-macro crate |
| `doido-kafka/src/lib.rs` | Module declarations |
| `doido-kafka/src/codec.rs` | `MessageCodec` trait + `JsonCodec` default |
| `doido-kafka/src/consumer.rs` | `Consumer` trait + `ConsumerContext` |
| `doido-kafka/tests/kafka_test.rs` | Unit tests (mocked) |

### Task 7.1 — `MessageCodec` trait (TDD)

- [ ] **Step 1:** Create crates + add to workspace (feature-gated with `kafka` feature).

- [ ] **Step 2:** Write failing test for `MessageCodec`: encode → decode roundtrip.

- [ ] **Step 3:** Implement `MessageCodec` trait + `JsonCodec` default impl.

- [ ] **Step 4:** Run — PASS.

- [ ] **Step 5:** Commit `feat(kafka): add MessageCodec trait and JsonCodec`

### Task 7.2 — `#[consumer]` + `#[topic]` macros (TDD)

- [ ] **Step 1:** Write failing test:

```rust
#[consumer(group = "myapp")]
struct EventConsumer;
impl EventConsumer {
    #[topic("user.created")]
    async fn on_user_created(&self, msg: Vec<u8>) -> doido_core::Result<()> { Ok(()) }
}
```

- [ ] **Step 2:** Implement macros generating topic → handler dispatch table.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(kafka): add #[consumer] and #[topic] macros`

### Task 7.3 — Integration test (mocked)

- [ ] **Step 1:** Create `doido-kafka/tests/kafka_test.rs` using `FakeConsumerContext` to simulate message dispatch.

- [ ] **Step 2:** Run — PASS.

- [ ] **Step 3:** Commit `test(kafka): integration test for consumer dispatch`

---

## Crate 8: `doido-mcp`

**Spec:** docs/14-mcp.md  
**Goal:** MCP server (HTTP + SSE) at `/mcp`; `#[tool]` on async fn; `#[resource]` on fn; `#[mcp_resource]` on sea-orm models; `mcp_server!` registration macro; typed + raw client.

### File Structure

| File | Purpose |
|------|---------|
| `doido-mcp/Cargo.toml` | Manifest |
| `doido-mcp-macros/Cargo.toml` | Proc-macro crate |
| `doido-mcp-macros/src/lib.rs` | `#[tool]`, `#[resource]`, `#[mcp_resource]`, `mcp_server!` |
| `doido-mcp/src/lib.rs` | Module declarations |
| `doido-mcp/src/protocol.rs` | JSON-RPC 2.0 types |
| `doido-mcp/src/registry.rs` | `ToolRegistry` + `ResourceRegistry` |
| `doido-mcp/src/server.rs` | Axum router at `/mcp` (HTTP + SSE) |
| `doido-mcp/src/client.rs` | Raw + typed client |
| `doido-mcp/tests/mcp_test.rs` | Integration tests |

### Task 8.1 — JSON-RPC protocol types + registry (TDD)

- [ ] **Step 1:** Create crates + add to workspace.

- [ ] **Step 2:** Write failing tests for `ToolRegistry`: register tool → list tools → call tool.

- [ ] **Step 3:** Implement `JsonRpcRequest/Response`, `ToolRegistry`, `ResourceRegistry`.

- [ ] **Step 4:** Run — PASS.

- [ ] **Step 5:** Commit `feat(mcp): add JSON-RPC types and tool/resource registries`

### Task 8.2 — `#[tool]` + `#[resource]` macros (TDD)

- [ ] **Step 1:** Write failing tests using the macros.

- [ ] **Step 2:** Implement macros generating `ToolDef` / `ResourceDef` descriptors registered via `mcp_server!`.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(mcp): add #[tool] and #[resource] proc-macros`

### Task 8.3 — Axum server + integration test (TDD)

- [ ] **Step 1:** Implement `server.rs` mounting `/mcp` with `tools/list`, `tools/call`, `resources/list`, `resources/read` endpoints.

- [ ] **Step 2:** Create integration test hitting endpoints via `tower::ServiceExt::oneshot`.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(mcp): add MCP server with HTTP endpoints and integration tests`

---

## Crate 9: `doido-generators`

**Spec:** docs/06b-generators.md  
**Goal:** `Generator` trait; registry of all Rails-equivalent targets (controller, model, migration, scaffold, job, mailer, channel, mcp_client); file writing; route auto-injection into `config/routes.rs`.

### File Structure

| File | Purpose |
|------|---------|
| `doido-generators/Cargo.toml` | Manifest |
| `doido-generators/src/lib.rs` | Module declarations + re-exports |
| `doido-generators/src/generator.rs` | `Generator` trait |
| `doido-generators/src/registry.rs` | Generator registry |
| `doido-generators/src/templates/` | Handlebars/minijinja templates for each target |
| `doido-generators/src/generators/` | One module per generator type |
| `doido-generators/tests/generators_test.rs` | Integration tests |

### Task 9.1 — `Generator` trait + registry (TDD)

- [ ] **Step 1:** Create `doido-generators/Cargo.toml` with minijinja (or similar), doido-core.

- [ ] **Step 2:** Write failing test for `Generator` trait:

```rust
struct FakeGenerator;
impl Generator for FakeGenerator {
    fn name(&self) -> &str { "fake" }
    fn generate(&self, args: &[&str]) -> doido_core::Result<Vec<GeneratedFile>> { Ok(vec![]) }
}
```

- [ ] **Step 3:** Implement `Generator` trait + `GeneratedFile` + `GeneratorRegistry`.

- [ ] **Step 4:** Run — PASS.

- [ ] **Step 5:** Commit `feat(generators): add Generator trait and registry`

### Task 9.2 — Individual generators (TDD, one commit per generator)

Implement and test each generator in order. Each generates correct file content for its target:

- [ ] `ControllerGenerator` — `app/controllers/<name>_controller.rs`
- [ ] `ModelGenerator` — `app/models/<name>.rs`
- [ ] `MigrationGenerator` — `db/migrations/<timestamp>_create_<name>.rs`
- [ ] `JobGenerator` — `app/jobs/<name>_job.rs`
- [ ] `MailerGenerator` — `app/mailers/<name>_mailer.rs`
- [ ] `ScaffoldGenerator` — controller + model + migration + views + routes
- [ ] `ChannelGenerator` — `app/channels/<name>_channel.rs`
- [ ] `McpClientGenerator` — `app/mcp/<name>_client.rs`

For each: write failing test → implement → verify PASS → commit.

### Task 9.3 — Route auto-injection (TDD)

- [ ] **Step 1:** Write failing test verifying that `ScaffoldGenerator` injects a `resources!` call into `config/routes.rs`.

- [ ] **Step 2:** Implement route-injection logic (read file, find insertion point, write back).

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(generators): add route auto-injection for scaffold generator`

---

## Crate 10: `doido-cli`

**Spec:** docs/06-cli.md  
**Goal:** Binary crate wiring all framework crates; clap-based CLI with commands: `server`, `console`, `routes`, `db migrate|rollback|seed|reset`, `jobs:failed|retry|discard`, `worker`, `credentials:edit`, `generate`.

### File Structure

| File | Purpose |
|------|---------|
| `doido-cli/Cargo.toml` | Manifest (binary crate) |
| `doido-cli/src/main.rs` | Entry point + clap setup |
| `doido-cli/src/commands/mod.rs` | Command module declarations |
| `doido-cli/src/commands/server.rs` | `doido server` |
| `doido-cli/src/commands/db.rs` | `doido db migrate|rollback|…` |
| `doido-cli/src/commands/jobs.rs` | `doido jobs:failed|retry|discard` |
| `doido-cli/src/commands/generate.rs` | `doido generate` → doido-generators |
| `doido-cli/src/commands/credentials.rs` | `doido credentials:edit` |
| `doido-cli/tests/cli_test.rs` | Integration tests (via `assert_cmd`) |

### Task 10.1 — Scaffold + help output (TDD)

- [ ] **Step 1:** Create `doido-cli/Cargo.toml` with clap, doido-generators, doido-model, doido-config, doido-jobs, doido-middleware.

- [ ] **Step 2:** Write failing test that runs `doido --help` via `assert_cmd` and checks exit code 0.

- [ ] **Step 3:** Implement minimal `main.rs` with clap subcommands.

- [ ] **Step 4:** Run — PASS.

- [ ] **Step 5:** Commit `feat(cli): add doido-cli scaffold with clap subcommands`

### Task 10.2 — `generate` command (TDD)

- [ ] **Step 1:** Write failing test: `doido generate controller Posts` creates a file in the expected path.

- [ ] **Step 2:** Implement `generate.rs` delegating to `GeneratorRegistry`.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(cli): wire generate command to doido-generators`

### Task 10.3 — `db` + `jobs` commands (TDD)

- [ ] **Step 1:** Write failing tests for `doido db migrate` and `doido jobs:failed`.

- [ ] **Step 2:** Implement `db.rs` and `jobs.rs` command handlers.

- [ ] **Step 3:** Run — PASS.

- [ ] **Step 4:** Commit `feat(cli): add db and jobs subcommands`

---

## Implementation Order & Dependencies

```
doido-model
    ↓
doido-cache     doido-middleware
    ↓
doido-jobs
    ↓
doido-mailer    doido-cable     doido-kafka     doido-mcp
    ↓
doido-generators
    ↓
doido-cli
```

Implement strictly in this order. Each crate's PR should have all tests green before the next crate begins.

## Completion Checklist

- [ ] doido-model — all tests green, committed
- [ ] doido-middleware — all tests green, committed
- [ ] doido-cache — all tests green, committed
- [ ] doido-jobs + doido-jobs-macros — all tests green, committed
- [ ] doido-mailer + doido-mailer-macros — all tests green, committed
- [ ] doido-cable + doido-cable-macros — all tests green, committed
- [ ] doido-kafka + doido-kafka-macros — all tests green, committed
- [ ] doido-mcp + doido-mcp-macros — all tests green, committed
- [ ] doido-generators — all tests green, committed
- [ ] doido-cli — all tests green, committed
- [ ] `cargo test --workspace` — all tests green
