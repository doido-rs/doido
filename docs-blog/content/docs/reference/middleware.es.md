+++
title = "Middleware y sesiones"
description = "La stack de middleware Tower, sesiones, flash, CSRF, CORS, rate limiting, health checks y rescue de errores."
weight = 5
+++

> **Especificación de diseño:** [`docs/07-middleware.md`](https://github.com/doido-rs/doido/blob/master/docs/07-middleware.md).
> Esta guía documenta la API tal como está implementada en `doido-controller`.

**Análogo en Rails: Rack + middleware.** Las preocupaciones transversales viven en una
stack de middleware Tower que envuelve tu router. El logging de peticiones y la
recuperación de panics están siempre activos; todo lo demás es opt-in. Las sesiones y los
mensajes flash son conectables y firmados.

## Vistazo general

```rust
use doido::controller::{MiddlewareStack, Session, SessionStore, CookieSessionStore, Flash};
```

## La stack de middleware

`MiddlewareStack` compone capas y envuelve un router con `apply()`. El **logging de
petición/respuesta** y la **recuperación de panics** (los panics se convierten en `500`)
se aplican siempre — la capa de logging queda en la parte más externa, así que también
registra los panics recuperados. Habilita el resto de forma fluida.

```rust
use doido::controller::MiddlewareStack;

let app = MiddlewareStack::new()
    .with_csrf()            // validación de token CSRF double-submit
    .with_cors()            // CORS permisivo (o with_cors_config(...))
    .with_force_ssl()       // redirige http→https (vía X-Forwarded-Proto)
    .with_allowed_hosts(vec!["example.com".into()]) // autorización de host
    .apply(router());       // envuelve tu router de routes!
```

Añade tus propias capas Tower en relación con las integradas con `insert_before` (dentro de
la stack) e `insert_after` (más externo).

## Sesiones

`SessionStore` es un trait async (`load`, `save`, `destroy`). El `CookieSessionStore` por
defecto es stateless y firmado con HMAC-SHA256; `CacheSessionStore` mantiene el estado en el
servidor en cualquier [cache store](@/docs/reference/cache.es.md). `Session` guarda valores
tipados vía `set`/`get`.

```rust
use doido::controller::{Session, CookieSessionStore, SessionStore};

let store = CookieSessionStore::new(secret_key_bytes);

let mut session = Session::new();
session.set("user_id", 42);
let uid: Option<i64> = session.get("user_id");

// Codifica directamente a un valor de cookie, o pasa por el trait async:
let cookie_value = store.encode(&session);
store.save(&session).await?;
let restored = store.decode(&cookie_value);
```

Sesiones en el servidor respaldadas por un cache store, con expiración:

```rust
use doido::controller::CacheSessionStore;

let store = CacheSessionStore::new(cache_store).with_ttl(3600); // 1 hora
```

## Mensajes flash

Mensajes de un solo uso que sobreviven exactamente a una redirección, transportados en una
cookie firmada.

```rust
use doido::controller::Flash;

let mut flash = Flash::new();
flash.set("notice", "Post created.");
let set_cookie = flash.to_cookie(&cookie_store);   // envía en la respuesta de redirección
// …siguiente petición:
let flash = Flash::from_cookie(&cookie_store, &raw_cookie);
for (key, message) in flash.iter() { /* exponer a la vista */ }
```

## Parámetros fuertes

La protección contra mass-assignment vive en el `Context` de la petición
(`query_params().require().permit()`). Ve la sección "Parámetros fuertes" en
[Controladores y enrutamiento](@/docs/reference/controllers.es.md).

## Protección CSRF

`with_csrf()` habilita la validación de token double-submit en las peticiones que cambian
estado.

```rust
let app = MiddlewareStack::new().with_csrf().apply(router());
```

## CORS

Habilita CORS permisivo, o contrólalo desde la sección de config `middleware.cors`
(`CorsConfig`: `enabled`, `allowed_origins`, `allowed_methods`).

```rust
use doido::controller::{MiddlewareStack, YamlConfig, Config};

let cfg = YamlConfig::load()?;
let app = MiddlewareStack::new()
    .with_cors_config(cfg.middleware().cors.clone())
    .apply(router());
```

## Rate limiting

`RateLimiter` cuenta peticiones por clave contra un límite y una ventana usando un
[cache store](@/docs/reference/cache.es.md) como contador de respaldo.

```rust
use doido::controller::rate_limit::RateLimiter;

let limiter = RateLimiter::new(cache_store, 100, 60); // 100 peticiones / 60s
```

## Health checks

`with_health()` monta un endpoint de health-check para balanceadores de carga y sondas de
uptime.

```rust
use doido::controller::health::with_health;

let app = with_health(router());
```

## Rescue de errores

`RescueHandlers` mapea tipos de error a respuestas — el análogo de `rescue_from` de Rails.

```rust
use doido::controller::RescueHandlers;

let handlers = RescueHandlers::new()
    .on::<NotFound>(|_e| /* Response 404 */ unimplemented!())
    .on::<Unauthorized>(|_e| /* Response 401 */ unimplemented!());

if let Some(response) = handlers.rescue(&err) {
    return response;
}
```

## Constraints de ruta

Valida parámetros de path antes de que corra la action, con validadores integrados
(`numeric`, `alpha`, `alphanumeric`, `uuid_like`) o los tuyos propios.

```rust
use doido::controller::constraints::{Constraints, numeric, uuid_like};

let constraints = Constraints::new()
    .param("id", numeric)
    .param("token", uuid_like);

let ok = constraints.matches(&[("id", "42"), ("token", "…")]);
```

## Especificación vs. implementación

> Las sesiones en cookie y flash están **firmadas** (HMAC-SHA256) pero aún no
> **cifradas** (AES-256-GCM está aplazado, junto con el trabajo de credenciales de la
> config). No guardes secretos en la cookie de sesión hasta que llegue el cifrado.

## Véase también

- [Controladores y enrutamiento](@/docs/reference/controllers.es.md) — donde la stack envuelve tus rutas.
- [Cache](@/docs/reference/cache.es.md) — el store detrás de `CacheSessionStore` y el rate limiting.
- [Configuración](@/docs/reference/configuration.es.md) — la sección `middleware.cors`.
