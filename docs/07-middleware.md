# doido-middleware — Spec

Rails analogue: **Rack middleware stack**

> **Status (2026-07-28): done, one gap.** The `MiddlewareStack` lives in
> **`doido-controller`** (this crate was merged in). Logging + panic recovery are
> always-on; CORS, CSRF, force-SSL, host-allowlist, request-id, and rate-limit are opt-in;
> `SessionStore` is pluggable (`CookieSessionStore` HMAC-signed + `CacheSessionStore`).
> Config is per-env **YAML**, not the TOML shown below (spec 05). **Open:** cookie session
> values are signed but **not AES-256-GCM encrypted** yet. See [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved in interview)

- **Request Logging and Panic Recovery are always on** — mandatory, cannot be disabled
- **All other middleware is opt-in** — enabled via `doido-config` or explicit `.layer()` call
- **Session is pluggable** via `SessionStore` trait; default impl is cookie-only (signed + encrypted)

## Default Stack (always active)

```
Request
  └─ PanicRecovery    ← catches panics, returns 500, never crashes the server
       └─ RequestLogger ← emits a tracing span per request (method, path, status, latency)
            └─ [opt-in layers]
                 └─ Router / Controller
```

## Opt-in Middleware

Enabled in `config/<env>.yml` under `middleware:` or registered manually:

| Middleware | Config key | Default |
|-----------|------------|---------|
| Request ID | `middleware.request_id = true` | false |
| CORS | `middleware.cors = { ... }` | disabled |
| Session | `middleware.session = { store = "cookie" }` | disabled |
| Compression | `middleware.compression = true` | false |
| Static files | `middleware.static_files = "public"` | disabled |
| Timeout | `middleware.timeout = 30` (seconds) | disabled |

Example `config/<env>.yml`:

```yaml
middleware:
  request_id: true
  compression: true
  timeout: 30
  cors:
    allowed_origins: ["https://myapp.com"]
    allowed_methods: ["GET", "POST", "PUT", "DELETE"]
  session:
    store: "cookie"              # "cookie" | "db" | "redis" | custom
    secret: "${SECRET_KEY_BASE}" # from credentials/env
  static_files:
    dir: "public"
```

## API-only mode (`doido new --api`)

A project generated with `--api` sets `api_only = true` under `[app]` in
`config/application.toml`. On boot, `server.rs` reads this marker and builds the
stack with `MiddlewareStack::with_api_only(true)`, which **skips HTML-only
middleware that makes no difference to `application/json` clients**:

| Layer | API mode |
|-------|----------|
| CSRF (form double-submit) | **skipped** even if `with_csrf()` is set — APIs use tokens/CORS |
| Session / flash / static-file serving | must honor the flag as they are wired (HTML-only) |
| Logging, panic recovery, CORS, force-SSL, host allowlist | unchanged (relevant to any content type) |

The same `api_only` marker is what the route macros read at compile time to drop
the `new`/`edit` form routes (see [01-router.md](01-router.md)).

## Session — Pluggable `SessionStore` Trait

```rust
pub trait SessionStore: Send + Sync {
    async fn load(&self, session_id: &str) -> Result<Option<SessionData>>;
    async fn save(&self, session_id: &str, data: &SessionData, ttl: Duration) -> Result<()>;
    async fn destroy(&self, session_id: &str) -> Result<()>;
}
```

Built-in implementations:

| Store | Crate feature | Description |
|-------|--------------|-------------|
| `CookieSessionStore` | default | Signed + AES-256-GCM encrypted cookie; no server state |
| `DbSessionStore` | `feature = "session-db"` | Session ID in cookie, data in sea-orm table |
| `RedisSessionStore` | `feature = "session-redis"` | Session ID in cookie, data in Redis |

Custom stores registered at boot:

```rust
doido_middleware::session::register(Box::new(MyCustomStore));
```

## Middleware Registration API

Three ways to add middleware:

```rust
// 1. Via config (declarative, recommended)
// config/<env>.yml → middleware: section (see above)

// 2. In the app router (programmatic)
routes! {
    middleware!(CorsLayer::new().allow_origin(Any));
    resources!(posts, PostsController);
}

// 3. Scoped to a namespace
routes! {
    namespace!(api, {
        middleware!(AuthLayer);
        resources!(users, Api::UsersController);
    });
}
```

## Known Requirements

- `PanicRecovery` and `RequestLogger` always compose first — cannot be removed
- `RequestLogger` emits structured tracing events: method, path, status, latency, request_id
- All opt-in middleware toggled via `doido-config` with no code changes
- `SessionStore` trait is the only session abstraction; no concrete type leaks into controllers
- Session accessed in controllers via `ctx.session.get::<T>(key)` / `ctx.session.set(key, val)`
- Middleware ordering in stack matches config declaration order for opt-in layers

## TDD Surface

- Test `PanicRecovery` returns 500 and logs error when action panics
- Test `RequestLogger` emits tracing span with correct fields
- Test each opt-in middleware in isolation (unit)
- Test CORS preflight returns correct headers when configured
- Test `CookieSessionStore`: set value, read back, survives request round-trip
- Test `DbSessionStore`: stores in DB, loads correctly, destroy removes row
- Test custom `SessionStore` impl integrates correctly
- Test middleware ordering: mandatory layers always outermost
- Test scoped middleware applies only within its namespace
- Integration test: full request through mandatory + opt-in stack, all layers fire in order
