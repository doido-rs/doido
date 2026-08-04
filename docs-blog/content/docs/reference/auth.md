+++
title = "Auth"
description = "Unified authentication — AuthUser trait, cookie/JWT/OAuth strategies, extractors, 2FA, and auth generators."
weight = 13
aliases = ['/docs/guides/auth/']

+++

> **Design spec:** [`docs/16-auth.md`](https://github.com/doido-rs/doido/blob/master/docs/16-auth.md).
> This guide documents the API as implemented in `doido-auth`.

**Rails analogue: Devise + OmniAuth + JWT.** `doido-auth` gives you a generic
`AuthUser` trait bound to your SeaORM model, pluggable **strategies** (cookie
session, JWT bearer, OAuth2), optional **2FA** (TOTP), axum **extractors**, and
**generators** that scaffold sessions, registration, passwords, and OAuth
controllers. Password hashing reuses `doido_model::password::HasSecurePassword`.

## At a glance

```rust
use doido::auth::{
    init, auth_layer, AuthUser, CurrentUser, MaybeUser, RequireAuth, AuthToken,
    mount, JwtStrategy, register_strategy,
};
```

Enable the crate via the `doido` meta crate:

```toml
[dependencies]
doido = { version = "0.0.9", features = ["auth"] }
```

## Getting started

The fastest path is a new app with auth baked in:

```bash
doido new blog --database=sqlite --auth
cd blog
cargo doido db create && cargo doido db migrate
cargo doido server
```

For an existing app, add the dependency first, then install:

```bash
cargo add doido --features auth   # or add doido-auth directly
cargo doido generate auth:install
cargo doido db migrate
```

`auth:install` emits the User migration and model, auth controllers, HTML views
(or JSON-only with `--api`), an `auth:` config snippet, and injects sign-in,
sign-up, password, and OAuth routes into `config/routes.rs`. It does **not**
modify `Cargo.toml` — the dependency must already be present.

## The `AuthUser` trait

Your User SeaORM model implements `AuthUser` — the generic contract for the
authenticated subject:

```rust
use doido::auth::AuthUser;
use doido::model::sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub email: String,
    #[sea_orm(column_name = "password_digest")]
    pub password_digest: String,
    pub two_factor_secret: Option<String>,
    pub two_factor_enabled: bool,
}

impl AuthUser for Model {
    type Id = i64;

    fn id(&self) -> Self::Id { self.id }
    fn email(&self) -> &str { &self.email }
    fn password_digest(&self) -> Option<&str> { Some(&self.password_digest) }

    async fn find_by_email(db: &DatabaseConnection, email: &str) -> doido::Result<Option<Self>> {
        Entity::find().filter(Column::Email.eq(email)).one(db).await
    }

    async fn find_by_id(db: &DatabaseConnection, id: Self::Id) -> doido::Result<Option<Self>> {
        Entity::find_by_id(id).one(db).await
    }
}
```

Combine with `HasSecurePassword` for credential verification. A manual `AuthUser`
impl is v1; a `#[auth_user]` derive is planned for a follow-up release.

## Configuration

The `auth` section of `config/<env>.yml` controls strategies, JWT, OAuth
providers, 2FA, and Devise-style route paths:

```yaml
auth:
  user_model: User
  strategies:
    - cookie                    # session cookie (default)
    - jwt                       # Authorization: Bearer <token>
  jwt:
    secret: "${JWT_SECRET}"
    access_ttl: 900              # 15 minutes
    refresh_ttl: 604800          # 7 days
    issuer: myapp
  oauth:
    idp:
      type: oauth2
      client_id: "${OAUTH_CLIENT_ID}"
      client_secret: "${OAUTH_CLIENT_SECRET}"
      redirect_uri: "/auth/idp/callback"
      authorize_url: "https://idp.example.com/oauth/authorize"
      token_url: "https://idp.example.com/oauth/token"
      scopes: [openid, email, profile]
  two_factor:
    enabled: false               # requires feature auth-2fa
    issuer: MyApp
  routes:
    prefix: /users
    sign_in: sign_in
    sign_out: sign_out
    sign_up: sign_up
    password_reset: password
```

Secrets come from `config/credentials.yml.enc` or environment variables. Enable
JWT in `strategies` only when `auth.jwt.secret` is set.

## Boot sequence

Initialise auth after the database pool and before serving requests:

```rust
use doido::auth::{init, auth_layer};

#[tokio::main]
async fn main() {
    let config = /* load config */;
    let db = /* connect pool */;

    init(db.clone(), &config.auth).await.expect("auth init");

    let app = config::routes::router()
        .layer(axum::middleware::from_fn(auth_layer));
    // …serve…
}
```

`init` registers enabled strategies, loads OAuth provider configs, and stores
process-global `AuthState`. `auth_layer` consults strategies in config order and
stores the first resolved `AuthIdentity` in request extensions.

## Strategies

| Strategy | Config name | How identity is resolved |
|----------|-------------|--------------------------|
| Cookie / session | `cookie` | Encrypted session cookie with `user_id` |
| JWT bearer | `jwt` | `Authorization: Bearer` header |
| Custom | any registered name | Your `AuthStrategy` impl |

Cookie session integrates with `doido_controller::session`. JWT issues signed
access + refresh token pairs via `JwtStrategy::issue_tokens`. Use both together
for HTML (session) and API (bearer) clients:

```rust
use doido::auth::{JwtStrategy, TokenPair};

let jwt = JwtStrategy::new(config.jwt.clone())?;
let tokens: TokenPair = jwt.issue_tokens(&serde_json::json!(user.id()))?;
```

Extractors consult strategies in config order; the first match wins.

## Axum extractors

| Extractor | Behaviour | HTTP status on failure |
|-----------|-----------|------------------------|
| `CurrentUser<U>` | Requires authenticated user, loaded from DB | `401 Unauthorized` |
| `MaybeUser<U>` | `Option<U>` — never fails | — |
| `RequireAuth` | Ensures identity without loading the full model | `401 Unauthorized` |
| `AuthToken` | Raw bearer token string | `401` if missing |

```rust
use doido::auth::extractors::{CurrentUser, MaybeUser};
use doido::controller::axum::Json;

async fn profile(CurrentUser(user): CurrentUser<User>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "email": user.email() }))
}

async fn home(MaybeUser(user): MaybeUser<User>) -> String {
    match user.0 {
        Some(u) => format!("Welcome back, {}", u.email()),
        None => "Guest".into(),
    }
}
```

Inside `#[controller]` actions, call `doido::auth::current_user::<User>(&ctx.parts())`
or use the extractors as handler parameters.

## Pre-built routes

`auth:install` injects explicit routes into `config/routes.rs`:

| Method | Path | Controller action |
|--------|------|-------------------|
| GET | `/users/sign_in` | `SessionsController::new` (HTML) |
| POST | `/users/sign_in` | `SessionsController::create` |
| DELETE | `/users/sign_out` | `SessionsController::destroy` |
| GET | `/users/sign_up` | `RegistrationsController::new` (HTML) |
| POST | `/users/sign_up` | `RegistrationsController::create` |
| POST | `/users/password` | `PasswordsController::create` |
| PATCH | `/users/password` | `PasswordsController::update` |
| GET | `/auth/{provider}` | `OauthController::authorize` |
| GET | `/auth/{provider}/callback` | `OauthController::callback` |

For programmatic mounting without generated controllers, use `mount`:

```rust
use doido::auth::mount;

// `create` persists a newly registered user (email + plaintext password):
let auth_router = mount::<User, _>(|db, email, password| {
    Box::pin(async move {
        User::create(&db, email, password).await // your app-specific helper
    })
});
```

Route paths honour `auth.routes` in config (`prefix`, `sign_in`, etc.).

## OAuth

Providers implement the `OAuthProvider` trait. Config entries with `type: oauth2`
become `OAuth2Provider` instances at boot; custom providers register at runtime:

```rust
use doido::auth::oauth::{OAuthProvider, OAuthTokenResponse, register_provider, get_provider};
use std::sync::Arc;

struct CustomProvider { /* … */ }

impl OAuthProvider for CustomProvider {
    fn name(&self) -> &str { "custom" }
    fn authorize_url(&self, state: &str) -> Result<String, AuthError> { /* … */ }
    fn exchange_code(&self, code: &str) -> Result<OAuthTokenResponse, AuthError> { /* … */ }
}

register_provider(Arc::new(CustomProvider { /* … */ }));

let provider = get_provider("idp").expect("idp configured");
let url = provider.authorize_url("random-state")?;
let tokens = provider.exchange_code("auth-code")?;
```

Callback routes are part of the generated `OauthController`. OAuth 1.0a config
entries are recognised but require a custom `OAuthProvider` impl.

## Two-factor authentication (feature `auth-2fa`)

Enable the feature and set `auth.two_factor.enabled: true`:

```toml
doido = { version = "0.0.9", features = ["auth", "auth-2fa"] }
```

```rust
use doido::auth::{enroll_two_factor, verify_two_factor_code, TwoFactorEnrollment};

let TwoFactorEnrollment { secret, otpauth_uri } = enroll_two_factor("user@example.com", "MyApp")?;
let ok = verify_two_factor_code(&secret, "123456")?;
```

`auth:install --two-factor` adds `two_factor_secret` and `two_factor_enabled`
columns plus 2FA controller/views. Backup-code storage is planned for a follow-up.

## Generators

Auth generators ship inside `doido-auth` and appear in `cargo doido generate`
**only when** `doido-auth` is listed in the project's `Cargo.toml`:

| Generator | Generates |
|-----------|-----------|
| `auth:install` | User migration + model, auth controllers, views, config, routes |
| `auth:controller <Name> actions…` | Controller with `CurrentUser` / auth guards |
| `auth:scaffold <Name> fields…` | Auth-aware scaffold with `user_id` ownership |

```bash
cargo doido generate auth:install
cargo doido generate auth:install --api          # JSON responses, no HTML views
cargo doido generate auth:install --two-factor     # 2FA columns + controllers
cargo doido generate auth:controller Dashboard index show
cargo doido generate auth:scaffold Post title:string body:text
```

Without `doido-auth` in `Cargo.toml`, auth generators are absent from the list
and dispatch returns an error pointing you to `doido new --auth` or `cargo add`.

## Custom strategies

Third-party backends (LDAP, SAML, magic link) implement `AuthStrategy`:

```rust
use doido::auth::{AuthStrategy, AuthIdentity, register_strategy};
use async_trait::async_trait;
use std::sync::Arc;

struct LdapStrategy { /* … */ }

#[async_trait]
impl AuthStrategy for LdapStrategy {
    fn name(&self) -> &str { "ldap" }

    async fn authenticate(&self, parts: &http::request::Parts, db: &DatabaseConnection)
        -> Result<Option<AuthIdentity>, doido::auth::AuthError>
    {
        // …
        Ok(None)
    }
}

register_strategy("ldap", Arc::new(LdapStrategy { /* … */ }));
```

Enable in config: `strategies: [cookie, ldap]`.

## Testing

`doido_auth::testing` provides in-memory fakes and helpers that serialise global
auth state across tests:

```rust
use doido_auth::testing::{AuthTestGuard, TestUser, seed_user, sign_in_request};

let _guard = AuthTestGuard::new();
seed_user(&db, "alice@example.com", "secret").await?;
let response = sign_in_request(&app, "alice@example.com", "secret").await?;
assert_eq!(response.status(), StatusCode::OK);
```

## Spec vs. implementation

> The design spec describes an `auth_routes!(User)` macro with Devise-style
> `only:`/`skip:` options. **v1 uses explicit routes** injected by `auth:install`
> (or `routes::mount` for programmatic apps). Refresh-token rotation tables,
> OAuth 1.0a token exchange, and the `#[auth_user]` derive are deferred.

## See also

- [Middleware & sessions](@/docs/reference/middleware.md) — the session store auth builds on.
- [Models](@/docs/reference/models.md) — `HasSecurePassword` for password digests.
- [Generators & CLI](@/docs/reference/generators.md) — auth generators and `doido new --auth`.
- [Cable](@/docs/reference/cable.md) — authorising WebSocket connections.
