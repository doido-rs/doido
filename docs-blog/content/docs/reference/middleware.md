+++
title = "Middleware & sessions"
description = "The Tower middleware stack, sessions, flash, CSRF, CORS, rate limiting, health checks, and error rescue."
weight = 5
aliases = ['/docs/guides/middleware/']

+++

> **Design spec:** [`docs/07-middleware.md`](https://github.com/doido-rs/doido/blob/master/docs/07-middleware.md).
> This guide documents the API as implemented in `doido-controller`.

**Rails analogue: Rack + middleware.** Cross-cutting concerns live in a Tower middleware
stack wrapped around your router. Request logging and panic recovery are always on;
everything else is opt-in. Sessions and flash messages are pluggable and signed.

## At a glance

```rust
use doido::controller::{MiddlewareStack, Session, SessionStore, CookieSessionStore, Flash};
```

## The middleware stack

`MiddlewareStack` composes layers and wraps a router with `apply()`. **Request/response
logging** and **panic recovery** (panics become `500`s) are always applied — the logging
layer sits outermost so it records recovered panics too. Opt into the rest fluently.

```rust
use doido::controller::MiddlewareStack;

let app = MiddlewareStack::new()
    .with_csrf()            // double-submit CSRF token validation
    .with_cors()            // permissive CORS (or with_cors_config(...))
    .with_force_ssl()       // redirect http→https (via X-Forwarded-Proto)
    .with_allowed_hosts(vec!["example.com".into()]) // host authorization
    .apply(router());       // wrap your routes! router
```

Add your own Tower layers relative to the built-in ones with `insert_before` (inside the
stack) and `insert_after` (outermost).

## Sessions

`SessionStore` is an async trait (`load`, `save`, `destroy`). The default
`CookieSessionStore` is stateless and signed with HMAC-SHA256; `CacheSessionStore` keeps
state server-side in any [cache store](@/docs/reference/cache.md). `Session` holds typed
values via `set`/`get`.

```rust
use doido::controller::{Session, CookieSessionStore, SessionStore};

let store = CookieSessionStore::new(secret_key_bytes);

let mut session = Session::new();
session.set("user_id", 42);
let uid: Option<i64> = session.get("user_id");

// Encode straight to a cookie value, or go through the async trait:
let cookie_value = store.encode(&session);
store.save(&session).await?;
let restored = store.decode(&cookie_value);
```

Server-side sessions backed by a cache store, with an expiry:

```rust
use doido::controller::CacheSessionStore;

let store = CacheSessionStore::new(cache_store).with_ttl(3600); // 1 hour
```

## Flash messages

One-shot messages that survive exactly one redirect, carried in a signed cookie.

```rust
use doido::controller::Flash;

let mut flash = Flash::new();
flash.set("notice", "Post created.");
let set_cookie = flash.to_cookie(&cookie_store);   // send on the redirect response
// …next request:
let flash = Flash::from_cookie(&cookie_store, &raw_cookie);
for (key, message) in flash.iter() { /* expose to the view */ }
```

## Strong parameters

Mass-assignment protection lives on the request `Context` (`query_params().require().permit()`).
See the "Strong parameters" section of [Controllers & routing](@/docs/reference/controllers.md).

## CSRF protection

`with_csrf()` enables double-submit token validation on state-changing requests.

```rust
let app = MiddlewareStack::new().with_csrf().apply(router());
```

## CORS

Enable permissive CORS, or drive it from the `middleware.cors` config section
(`CorsConfig`: `enabled`, `allowed_origins`, `allowed_methods`).

```rust
use doido::controller::{MiddlewareStack, YamlConfig, Config};

let cfg = YamlConfig::load()?;
let app = MiddlewareStack::new()
    .with_cors_config(cfg.middleware().cors.clone())
    .apply(router());
```

## Rate limiting

`RateLimiter` counts requests per key against a limit and window using a
[cache store](@/docs/reference/cache.md) as the backing counter.

```rust
use doido::controller::rate_limit::RateLimiter;

let limiter = RateLimiter::new(cache_store, 100, 60); // 100 requests / 60s
```

## Health checks

`with_health()` mounts a health-check endpoint for load balancers and uptime probes.

```rust
use doido::controller::health::with_health;

let app = with_health(router());
```

## Error rescue

`RescueHandlers` maps error types to responses — the Rails `rescue_from` analogue.

```rust
use doido::controller::RescueHandlers;

let handlers = RescueHandlers::new()
    .on::<NotFound>(|_e| /* 404 Response */ unimplemented!())
    .on::<Unauthorized>(|_e| /* 401 Response */ unimplemented!());

if let Some(response) = handlers.rescue(&err) {
    return response;
}
```

## Route constraints

Validate path parameters before an action runs, with built-in validators (`numeric`,
`alpha`, `alphanumeric`, `uuid_like`) or your own.

```rust
use doido::controller::constraints::{Constraints, numeric, uuid_like};

let constraints = Constraints::new()
    .param("id", numeric)
    .param("token", uuid_like);

let ok = constraints.matches(&[("id", "42"), ("token", "…")]);
```

## Spec vs. implementation

> Cookie sessions and flash are **signed** (HMAC-SHA256) but not yet **encrypted**
> (AES-256-GCM is deferred, tracked with the config credentials work). Don't store secrets
> in the session cookie until encryption lands.

## See also

- [Controllers & routing](@/docs/reference/controllers.md) — where the stack wraps your routes.
- [Cache](@/docs/reference/cache.md) — the store behind `CacheSessionStore` and rate limiting.
- [Configuration](@/docs/reference/configuration.md) — the `middleware.cors` section.
