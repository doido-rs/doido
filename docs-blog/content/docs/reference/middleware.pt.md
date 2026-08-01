+++
title = "Middleware & sessões"
description = "A stack de middleware Tower, sessões, flash, CSRF, CORS, rate limiting, health checks e rescue de erros."
weight = 5
+++

> **Especificação de design:** [`docs/07-middleware.md`](https://github.com/doido-rs/doido/blob/master/docs/07-middleware.md).
> Este guia documenta a API como implementada em `doido-controller`.

**Análogo no Rails: Rack + middleware.** Preocupações transversais vivem em uma stack de
middleware Tower que envolve o seu router. Logging de requisições e recuperação de panic
estão sempre ligados; todo o resto é opt-in. Sessões e mensagens flash são plugáveis e
assinadas.

## Visão geral

```rust
use doido_controller::{MiddlewareStack, Session, SessionStore, CookieSessionStore, Flash};
```

## A stack de middleware

`MiddlewareStack` compõe camadas e envolve um router com `apply()`. **Logging de
requisição/resposta** e **recuperação de panic** (panics viram `500`) são sempre aplicados
— a camada de logging fica na parte mais externa, então também registra panics
recuperados. Habilite o resto de forma fluente.

```rust
use doido_controller::MiddlewareStack;

let app = MiddlewareStack::new()
    .with_csrf()            // validação de token CSRF double-submit
    .with_cors()            // CORS permissivo (ou with_cors_config(...))
    .with_force_ssl()       // redireciona http→https (via X-Forwarded-Proto)
    .with_allowed_hosts(vec!["example.com".into()]) // autorização de host
    .apply(router());       // envolve o seu router do routes!
```

Adicione suas próprias camadas Tower em relação às embutidas com `insert_before` (dentro da
stack) e `insert_after` (mais externo).

## Sessões

`SessionStore` é um trait async (`load`, `save`, `destroy`). O `CookieSessionStore` padrão
é stateless e assinado com HMAC-SHA256; `CacheSessionStore` mantém o estado no servidor em
qualquer [cache store](@/docs/reference/cache.pt.md). `Session` guarda valores tipados via
`set`/`get`.

```rust
use doido_controller::{Session, CookieSessionStore, SessionStore};

let store = CookieSessionStore::new(secret_key_bytes);

let mut session = Session::new();
session.set("user_id", 42);
let uid: Option<i64> = session.get("user_id");

// Codifique direto para um valor de cookie, ou passe pelo trait async:
let cookie_value = store.encode(&session);
store.save(&session).await?;
let restored = store.decode(&cookie_value);
```

Sessões no servidor apoiadas por um cache store, com expiração:

```rust
use doido_controller::CacheSessionStore;

let store = CacheSessionStore::new(cache_store).with_ttl(3600); // 1 hora
```

## Mensagens flash

Mensagens de uso único que sobrevivem a exatamente um redirect, carregadas em um cookie
assinado.

```rust
use doido_controller::Flash;

let mut flash = Flash::new();
flash.set("notice", "Post created.");
let set_cookie = flash.to_cookie(&cookie_store);   // envia na resposta de redirect
// …próxima requisição:
let flash = Flash::from_cookie(&cookie_store, &raw_cookie);
for (key, message) in flash.iter() { /* expõe para a view */ }
```

## Parâmetros fortes

A proteção contra mass-assignment vive no `Context` da requisição
(`query_params().require().permit()`). Veja a seção "Parâmetros fortes" em
[Controllers & roteamento](@/docs/reference/controllers.pt.md).

## Proteção CSRF

`with_csrf()` habilita a validação de token double-submit em requisições que mudam estado.

```rust
let app = MiddlewareStack::new().with_csrf().apply(router());
```

## CORS

Habilite CORS permissivo, ou controle pela seção de config `middleware.cors` (`CorsConfig`:
`enabled`, `allowed_origins`, `allowed_methods`).

```rust
use doido_controller::{MiddlewareStack, YamlConfig, Config};

let cfg = YamlConfig::load()?;
let app = MiddlewareStack::new()
    .with_cors_config(cfg.middleware().cors.clone())
    .apply(router());
```

## Rate limiting

`RateLimiter` conta requisições por chave contra um limite e uma janela usando um
[cache store](@/docs/reference/cache.pt.md) como contador de apoio.

```rust
use doido_controller::rate_limit::RateLimiter;

let limiter = RateLimiter::new(cache_store, 100, 60); // 100 requisições / 60s
```

## Health checks

`with_health()` monta um endpoint de health-check para load balancers e sondas de uptime.

```rust
use doido_controller::health::with_health;

let app = with_health(router());
```

## Rescue de erros

`RescueHandlers` mapeia tipos de erro para respostas — o análogo ao `rescue_from` do Rails.

```rust
use doido_controller::RescueHandlers;

let handlers = RescueHandlers::new()
    .on::<NotFound>(|_e| /* Response 404 */ unimplemented!())
    .on::<Unauthorized>(|_e| /* Response 401 */ unimplemented!());

if let Some(response) = handlers.rescue(&err) {
    return response;
}
```

## Constraints de rota

Valide parâmetros de path antes de a action rodar, com validadores embutidos (`numeric`,
`alpha`, `alphanumeric`, `uuid_like`) ou os seus próprios.

```rust
use doido_controller::constraints::{Constraints, numeric, uuid_like};

let constraints = Constraints::new()
    .param("id", numeric)
    .param("token", uuid_like);

let ok = constraints.matches(&[("id", "42"), ("token", "…")]);
```

## Especificação vs. implementação

> Sessões em cookie e flash são **assinadas** (HMAC-SHA256), mas ainda não **criptografadas**
> (AES-256-GCM está adiado, acompanhando o trabalho de credenciais da config). Não guarde
> segredos no cookie de sessão até a criptografia chegar.

## Veja também

- [Controllers & roteamento](@/docs/reference/controllers.pt.md) — onde a stack envolve suas rotas.
- [Cache](@/docs/reference/cache.pt.md) — o store por trás de `CacheSessionStore` e do rate limiting.
- [Configuração](@/docs/reference/configuration.pt.md) — a seção `middleware.cors`.
