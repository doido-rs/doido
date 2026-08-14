# 16 — Auth (`doido-auth`)

Unified authentication for Doido — the **Devise + OmniAuth + JWT** analogue. It
provides a generic `AuthUser` trait bound to your SeaORM model, pluggable
**strategies** (cookie session, JWT bearer, OAuth/OAuth2), optional **2FA** (TOTP),
and **pre-built session/registration routes** that work after a single line in
`config/routes.rs`. Axum **extractors** (`CurrentUser`, `MaybeUser`, `RequireAuth`)
compose with the existing `doido-controller` session stack and `#[controller]`
filters.

> **Status (2026-08-14): implemented, built-in-by-default.** Crate `doido-auth` ships with
> `AuthUser`, cookie/JWT/OAuth strategies, optional 2FA (`auth-2fa`), axum extractors,
> `routes::mount`, and generators (`auth:install` / `auth:controllers` / `auth:controller` /
> `auth:scaffold`). Generators appear in `doido generate` only when `doido-auth` is a project
> dependency. Password hashing lives in `doido-model::password`; auth builds on it.
>
> **Devise-style modules.** `auth.modules` in `config/<env>.yml` selects which features are
> active (`database_authenticatable`, `registerable`, `recoverable`, `rememberable`,
> `trackable`, `timeoutable`, `validatable`, `confirmable`, `lockable`, `omniauthable`,
> `two_factor_authenticatable`). `auth:install` is module-aware — it emits the matching
> migration columns/entity fields, writes the `modules:` list, and (with `--modules=`)
> restricts mounted routes via `auth_routes!(User, only: […])`. See [Modules](#modules).
>
> **Controllers are built-in, not copied.** `doido new --auth` / `auth:install` wire a bare
> `auth_routes!(User)` onto doido-auth's built-in controllers and render HTML from
> framework-provided, overridable views — **no controllers or views are written into the
> app**. Run `doido generate auth:controllers` to *eject* them for customization (the
> `devise:controllers` + `devise:views` analogue). Release e2e: `auth_generators` (built-in
> default + eject) and `auth_install` (API/HTML flows), run via `make release-e2e`.

## Crate map

| Module | Responsibility |
|--------|----------------|
| `user` | `AuthUser` trait — the generic contract your User model implements |
| `config` | `auth:` section of `config/<env>.yml` → `AuthConfig` (strategies, 2FA, OAuth clients) |
| `session` | Cookie/session strategy — integrates with `doido_controller::session` |
| `jwt` | JWT bearer strategy — sign/verify access + refresh tokens |
| `oauth` | `OAuthProvider` trait, config-backed `OAuth2Provider`, provider registry |
| `two_factor` | Optional TOTP 2FA — enroll, verify, backup codes (feature `auth-2fa`) |
| `extractors` | Axum `FromRequestParts` impls: `CurrentUser`, `MaybeUser`, `RequireAuth` |
| `routes` | Pre-built `AuthRoutes` controller + route table for sessions/registration/2FA |
| `generators` | **`auth:install` / `auth:controller` / `auth:scaffold`** — owned by this crate; registered into the CLI only when the app depends on `doido-auth` |
| `registry` | Custom strategy registration (`register_strategy`) |
| `testing` | In-memory fakes, test helpers, signed-token fixtures |

Proc-macro crate **`doido-auth/macros`** (optional v1 follow-up): `#[auth_user]`
derives `AuthUser` from a SeaORM model with conventional column names.

## Rails analogue

| Doido | Rails |
|-------|-------|
| `doido generate auth:install` | `rails generate devise:install` + `devise User` |
| `AuthRoutes` (sessions/registration) | Devise routes (`/users/sign_in`, `/users/sign_up`, …) |
| `CurrentUser` extractor | `current_user` helper in controllers |
| `MaybeUser` extractor | `user_signed_in?` + optional user |
| OAuth providers | OmniAuth strategies |
| JWT strategy | `devise-jwt` / doorkeeper-style bearer tokens |
| 2FA (TOTP) | Devise two-factor / ROTP |
| `auth:scaffold` | Devise + scaffold for a resource with auth-aware CRUD |

## Decisions (resolved in interview)

- **Separate crate** — `doido-auth` is independently usable and testable; not merged
  into `doido-controller` (sessions stay in controller; auth *uses* them).
- **Generic user model** — apps configure one SeaORM entity as the auth subject via
  `AuthUser`; no hard-coded User struct in the framework.
- **Strategies are pluggable and composable** — cookie session is the default;
  JWT and OAuth are opt-in via config; multiple strategies can be active (e.g.
  session for HTML, JWT for API).
- **Extractors, not globals** — `CurrentUser<U>` is an axum extractor; no thread-local
  `current_user()` magic (controllers can still use `#[before_action]` wrappers).
- **Pre-built routes** — session sign-in/out, registration, password reset, OAuth
  callbacks, and 2FA challenge endpoints ship as a mountable route group; apps only
  register `auth_routes!(User);` in `config/routes.rs`.
- **Password hashing** — reuse `doido_model::password` (`HasSecurePassword`); auth
  does not re-implement bcrypt.
- **2FA is optional** — behind feature `auth-2fa`; disabled by default in generated apps.
- **Generators live in this crate** — `auth:install`, `auth:controller`, and
  `auth:scaffold` are implemented under `doido-auth/src/generators/`, **not** in
  `doido-generators`. They appear in `doido generate` **if and only if** the current
  project's `Cargo.toml` lists `doido-auth` as a dependency (directly or via the
  `doido` meta crate with the `auth` feature). Without that dependency the auth
  generators are absent from the list and dispatch returns *unknown generator*.
- **Bootstrap** — `doido new --auth` adds `doido-auth` to the generated app and runs
  `auth:install`; for existing apps, `cargo add doido-auth` (or equivalent) must
  happen before auth generators become available.

## `AuthUser` trait

Your User model implements this trait (manually or via `#[auth_user]`):

```rust
use doido_auth::AuthUser;
use doido_model::sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub email: String,
    #[sea_orm(column_name = "password_digest")]
    pub password_digest: String,
    pub two_factor_secret: Option<String>,  // when 2FA enabled
    pub two_factor_enabled: bool,
}

impl AuthUser for Model {
    type Id = i64;

    fn id(&self) -> Self::Id { self.id }
    fn email(&self) -> &str { &self.email }
    fn password_digest(&self) -> Option<&str> { Some(&self.password_digest) }

    // Optional overrides (defaults use conventional column names above)
    fn find_by_email(db: &DatabaseConnection, email: &str) -> impl Future<Output = Result<Option<Self>>> + Send {
        Entity::find().filter(Column::Email.eq(email)).one(db)
    }
    fn find_by_id(db: &DatabaseConnection, id: Self::Id) -> impl Future<Output = Result<Option<Self>>> + Send {
        Entity::find_by_id(id).one(db)
    }
}
```

`AuthUser` requires `Clone + Send + Sync + 'static` and integrates with
`doido_model::password::HasSecurePassword` for credential verification.

## Configuration

Configured in the `auth` section of `config/<env>.yml`:

```yaml
auth:
  user_model: User                    # Rust type name (for codegen hints)
  strategies:
    - cookie                          # default — uses controller session store
    - jwt                             # Authorization: Bearer <token>
  jwt:
    secret: "${JWT_SECRET}"            # or credentials
    access_ttl: 900                    # seconds (15 min)
    refresh_ttl: 604800                # seconds (7 days)
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
    legacy:
      type: oauth1
      consumer_key: "${OAUTH1_KEY}"
      consumer_secret: "${OAUTH1_SECRET}"
  two_factor:
    enabled: false                     # opt-in; requires feature auth-2fa
    issuer: MyApp                      # TOTP label
  routes:
    prefix: /users                     # Devise-style path prefix
    sign_in: sign_in
    sign_out: sign_out
    sign_up: sign_up
    password_reset: password           # /users/password
```

Credentials come from `config/credentials.yml.enc` or environment variables and
must not be committed.

## Modules

`auth.modules` is the Devise `devise :database_authenticatable, …` analogue: a list
that selects which auth features are active. A module governs its **routes** (via the
generated `auth_routes!` `only:` list), its **migration columns / entity fields**, and
its **runtime behavior**. `strategies` (cookie/JWT) are orthogonal — they decide *how* a
request is authenticated, modules decide *which* Devise features exist.

```yaml
auth:
  modules:
    - database_authenticatable   # password auth (required)
    - registerable               # sign-up
    - recoverable                # password reset
    - rememberable               # remember-me cookie
    - validatable                # email/password validation
  timeout: 1800                  # timeoutable: idle seconds
  password_length: 6             # validatable: minimum length
  maximum_attempts: 20           # lockable: fails before lock
```

Defaults to Devise's default set (`database_authenticatable`, `registerable`,
`recoverable`, `rememberable`, `validatable`). `auth:install --modules=a,b,c` selects an
explicit set: it emits the matching migration columns + entity fields, writes the
`modules:` block, and restricts mounted routes with `auth_routes!(User, only: [...])`.
With no `--modules`, install keeps the permissive bare `auth_routes!(User)`.

### Devise ↔ doido-auth module parity

| Devise module | doido module | Migration columns | Routes | Status |
|---------------|--------------|-------------------|--------|--------|
| database_authenticatable | `database_authenticatable` | `email`, `password_digest` | `sessions` | ✅ implemented |
| registerable | `registerable` | — | `registrations` | ✅ implemented |
| omniauthable | `omniauthable` | — | `oauth` | ✅ implemented (OAuth2) |
| validatable | `validatable` | — | — | ✅ implemented (email + length) |
| trackable | `trackable` | `sign_in_count`, `current/last_sign_in_at`, `current/last_sign_in_ip` | — | ✅ implemented (recorded on sign-in) |
| two_factor | `two_factor_authenticatable` | `two_factor_secret`, `two_factor_enabled` | `two_factor` | ⚙️ TOTP core; enroll/challenge stubs (`auth-2fa`) |
| recoverable | `recoverable` | `reset_password_token`, `reset_password_sent_at` | `passwords` (`new`/`create`/`edit`/`update`) | ✅ token reset + email via doido-mailer (`reset_password_within`) |
| rememberable | `rememberable` | `remember_created_at` | — | ✅ persistent signed remember cookie + `RememberStrategy` auto-login (`remember_for`) |
| timeoutable | `timeoutable` | — | — | ✅ absolute session-age expiry (`auth.timeout`); idle-reset pending |
| confirmable | `confirmable` | `confirmation_token`, `confirmed_at`, `confirmation_sent_at`, `unconfirmed_email` | `confirmation` (`show`/`create`) | ✅ email confirmation + sign-in gating (registration sends confirmation; unconfirmed sign-in → 403) |
| lockable | `lockable` | `failed_attempts`, `unlock_token`, `locked_at` | `unlock` | ✅ lock after `maximum_attempts`, time-based auto-unlock (`unlock_in`); email unlock pending |

Legend: ✅ full runtime behavior · ⚙️ config + schema + routing recognized; some runtime
behavior is a tracked follow-up. Every module is selectable and behavioral today except
`two_factor_authenticatable`, whose TOTP enroll/challenge handlers remain stubs (feature
`auth-2fa`). All behaviors live in the built-in handlers and are gated on `auth.modules`,
so they work whether you use the built-in controllers or eject with `auth:controllers`.

**Config knobs.** `password_length` (validatable), `timeout` (timeoutable),
`maximum_attempts`/`unlock_in` (lockable), `reset_password_within` (recoverable),
`remember_for` (rememberable).

## Strategies

### Cookie / session (default)

Uses the existing `doido_controller::session` stack. On successful sign-in, auth
stores `user_id` (typed via `AuthUser::Id`) in the session. The `CurrentUser`
extractor loads the user from the DB on each request.

```rust
// Boot (generated by auth:install)
doido_auth::init(&db, AuthConfig::from_yaml(&config)).await?;
```

### JWT bearer

For API-only or SPA clients. Issues signed access + refresh token pairs; the
`Authorization: Bearer` header is parsed by the JWT strategy before the cookie
strategy. Refresh tokens can be rotated and stored server-side (optional table
emitted by `auth:install`).

```rust
// Protected API route — JWT or session both work when both strategies enabled
async fn profile(CurrentUser(user): CurrentUser<User>) -> impl IntoResponse {
    Json(Profile { email: user.email().to_string() })
}
```

### OAuth

Providers implement the [`OAuthProvider`](doido-auth/src/oauth.rs) trait. Config
entries with `type: oauth2` become [`OAuth2Provider`](doido-auth/src/oauth.rs)
instances at boot; custom providers register via `register_provider`:

```rust
use doido_auth::oauth::{OAuthProvider, OAuthTokenResponse, register_provider};
use std::sync::Arc;

struct CustomProvider { /* … */ }

impl OAuthProvider for CustomProvider {
    fn name(&self) -> &str { "custom" }
    fn authorize_url(&self, state: &str) -> Result<String, AuthError> { /* … */ }
    fn exchange_code(&self, code: &str) -> Result<OAuthTokenResponse, AuthError> { /* … */ }
}

register_provider(Arc::new(CustomProvider { /* … */ }));
```

Callback routes are part of `AuthRoutes` (`GET /auth/:provider/callback`).

## Axum extractors

All extractors live in `doido_auth::extractors` and work with `doido_controller::axum`:

| Extractor | Behaviour | HTTP status on failure |
|-----------|-----------|------------------------|
| `CurrentUser<U>` | Requires authenticated user via any enabled strategy | `401 Unauthorized` |
| `MaybeUser<U>` | `Option<U>` — never fails | — |
| `RequireAuth` | Ensures *some* identity without loading the full model | `401 Unauthorized` |
| `AuthToken` | Raw bearer/JWT string (for token refresh endpoints) | `401` if missing |

Extractors consult strategies in config order; the first strategy that resolves an
identity wins.

```rust
use doido_auth::extractors::{CurrentUser, MaybeUser};
use doido_controller::axum::{Router, routing::get};

async fn dashboard(CurrentUser(user): CurrentUser<User>) -> impl IntoResponse {
    format!("Hello, {}", user.email())
}

async fn home(MaybeUser(user): MaybeUser<User>) -> impl IntoResponse {
    match user {
        Some(u) => format!("Welcome back, {}", u.email()),
        None => "Guest".into(),
    }
}
```

Inside `#[controller]` actions, use the same types as handler parameters or call
`doido_auth::current_user::<User>(&ctx)` which reads from request extensions set
by the auth middleware layer.

## Optional 2FA (feature `auth-2fa`)

When enabled in config and the feature is compiled in:

1. **Enroll** — `POST /users/two_factor` generates a TOTP secret + QR URI.
2. **Confirm** — `POST /users/two_factor/confirm` verifies a code and sets
   `two_factor_enabled`.
3. **Challenge** — after password sign-in, users with 2FA enabled receive a
   `422` + redirect to `/users/two_factor/challenge`; session holds a pending flag
   until the TOTP code is verified.
4. **Backup codes** — one-time recovery codes stored hashed in `user_backup_codes`.

Disable the feature entirely for API-only JWT apps that delegate 2FA upstream.

## Pre-built routes (`AuthRoutes`)

Mount with one line in `config/routes.rs`:

```rust
routes! {
    get!("/", HomeController::index);
    auth_routes!(User);   // ← sessions, registration, OAuth callbacks, 2FA
    resources!(posts, PostsController);
}
```

Default route table (prefix `/users`):

| Method | Path | Action | Description |
|--------|------|--------|-------------|
| GET | `/users/sign_in` | `new` | Sign-in form (HTML) or 401 hint (API) |
| POST | `/users/sign_in` | `create` | Authenticate (password + optional 2FA step) |
| DELETE | `/users/sign_out` | `destroy` | Sign out — clears session + revokes JWT |
| GET | `/users/sign_up` | `registration#new` | Registration form |
| POST | `/users/sign_up` | `registration#create` | Create account |
| GET | `/users/password/new` | `password#new` | Request reset email |
| POST | `/users/password` | `password#create` | Send reset token |
| PATCH | `/users/password` | `password#update` | Set new password with token |
| GET | `/auth/:provider` | `oauth#redirect` | Start OAuth flow |
| GET | `/auth/:provider/callback` | `oauth#callback` | OAuth callback |
| GET | `/users/two_factor/new` | `two_factor#new` | Enroll 2FA (feature) |
| POST | `/users/two_factor` | `two_factor#create` | Enable 2FA |
| POST | `/users/two_factor/challenge` | `two_factor#challenge` | Verify TOTP after sign-in |

`auth_routes!` accepts options mirroring Devise:

```rust
auth_routes!(User, only: [sessions, registrations]);
auth_routes!(User, skip: [passwords, two_factor]);
auth_routes!(User, prefix: "/accounts");
```

Controllers and views are generated by `auth:install`; routes reference them by
convention (`Auth::SessionsController`, etc.).

## Boot sequence integration

After config and DB pool init, before the HTTP server:

```rust
// src/main.rs (generated)
doido_auth::init(
    doido_model::pool::connection(),
    &config.auth,
).await?;
```

`init` registers enabled strategies, loads OAuth provider configs, and installs
the auth middleware layer on the axum router (via `doido_auth::layer()`).

## Generators (crate-owned, conditionally visible)

Auth generators are **not** built into `doido-generators`. They ship inside
`doido-auth` and register through `doido_auth::generators::register(&mut registry)`.
The CLI merges them at runtime only when the project's `Cargo.toml` declares a
`doido-auth` dependency:

```toml
[dependencies]
doido-auth = "0.0.9"
# or, in generated apps:
doido = { version = "0.0.9", features = ["auth"] }
```

```rust
// doido-auth/src/generators/mod.rs — called by the CLI when the dep is present
use doido_generators::GeneratorRegistry;

pub fn register(reg: &mut GeneratorRegistry) {
    reg.register(Box::new(AuthInstallGenerator));
    reg.register(Box::new(AuthControllerGenerator));
    reg.register(Box::new(AuthScaffoldGenerator));
}
```

```rust
// doido-generators/src/commands/generate.rs (conceptual)
fn registry_for_project() -> GeneratorRegistry {
    let mut reg = default_registry();
    if project_has_doido_auth("Cargo.toml") {
        doido_auth::generators::register(&mut reg);
    }
    reg
}
```

`doido generate` with no arguments lists auth generators under a separate heading
**only when installed**:

```
Available generators:

Built-in:
  controller
  model
  …

Auth (doido-auth):        ← omitted entirely when doido-auth is not a dependency
  auth:install
  auth:controller
  auth:scaffold
```

Invoking `doido generate auth:install` without `doido-auth` in `Cargo.toml` fails
with an error that tells the user to add the dependency or run `doido new --auth`.

### Bootstrap paths

| Situation | How to get auth generators |
|-----------|---------------------------|
| New app with auth | `doido new myapp --database=sqlite --auth` — adds `doido-auth` to `Cargo.toml` and runs `auth:install` |
| Existing app | `cargo add doido-auth` (or add to `Cargo.toml` manually), then `doido generate auth:install` |
| Remove auth | Remove `doido-auth` from `Cargo.toml`; auth generators disappear from `doido generate` |

`auth:install` wires app files (migration, controllers, config, routes) but does
**not** add the crate dependency — that must already be present (or come from
`doido new --auth`, which adds the dep first).

| Generator | Files created | Route injected |
|-----------|---------------|----------------|
| `auth:install` | User migration + model + `auth:` config; **no** controllers/views (uses built-ins) | Yes — bare `auth_routes!(User);` (or `only:` with `--modules=`) |
| `auth:controllers` | Ejects `app/controllers/auth/**` + `app/views/auth/**`, rewires routes to local controllers (`--api`/`--two-factor`/`--controllers-only`/`--views-only`) | Rewrites `auth_routes!(User, controllers: { … })` |
| `auth:controller <Name>` | Controller with `CurrentUser` / `before_action` auth guards | Yes — REST or custom |
| `auth:scaffold <Name> fields…` | `auth:install` (if missing) + model + migration + auth-aware scaffold | Yes — `resources!(...)` + auth |

Module layout:

```
doido-auth/
  src/
    generators/
      mod.rs              ← register() exports all three generators
      install.rs          ← auth:install
      controller.rs       ← auth:controller
      scaffold.rs         ← auth:scaffold
    templates/            ← embedded via include_str! (same pattern as doido-generators)
      user.rs.tera
      sessions_controller.rs.tera
      …
```

Generators depend on `doido-generators` for the `Generator` trait and route-injection
helpers (`route_injector`), not the other way around.

### `auth:install`

The `devise:install` + `devise User` analogue. It wires auth onto the framework's
**built-in** controllers and views — it does **not** copy any controllers/views into
the app (run [`auth:controllers`](#authcontrollers-eject) to eject them):

```sh
doido generate auth:install
doido generate auth:install --two-factor                 # 2FA columns + module
doido generate auth:install --modules=database_authenticatable,registerable,confirmable,lockable
```

Produces:
- Migration: `users` table (`email`, `password_digest`, `created_at`, `updated_at`) plus
  the columns of each enabled [module](#modules) (e.g. `reset_password_token`,
  `confirmation_token`, `failed_attempts`, `sign_in_count`, …).
- `app/models/user.rs` implementing `AuthUser` + `HasSecurePassword` (+ `RegisterableAuthUser`)
  and `app/models/_entities/users.rs` with the module fields.
- Appends an `auth:` block (with the `modules:` list) to `config/development.yml` /
  `config/test.yml`.
- Injects `use crate::models::user::Model as User;` and a bare `auth_routes!(User);` (or
  `auth_routes!(User, only: [...])` with `--modules=`) into `config/routes.rs`. Targets
  doido-auth's built-in controllers; does **not** modify `Cargo.toml`.

### `auth:controllers` (eject)

The `devise:controllers` + `devise:views` analogue. Copies the auth controllers and views
into the app for customization and rewires `config/routes.rs` to point `auth_routes!` at
the local controllers (`controllers: { sessions: auth::SessionsController, … }`, dropping
the now-unused `User` import):

```sh
doido generate auth:controllers                  # controllers + HTML views
doido generate auth:controllers --api            # controllers only (JSON)
doido generate auth:controllers --views-only     # just the views (built-in controllers stay)
doido generate auth:controllers --controllers-only
```

### `auth:controller`

Like `generate controller`, but actions assume an authenticated subject:

```sh
doido generate auth:controller Dashboard index show
```

Generated controller includes:

```rust
#[controller]
struct DashboardController;

impl DashboardController {
    #[before_action(require_user)]  // generated filter using CurrentUser
    async fn index(ctx: &mut Context, user: CurrentUser<User>) -> Result<Response> { … }
}
```

### `auth:scaffold`

Combines `auth:install` (when not present), `scaffold`, and auth guards:

```sh
doido generate auth:scaffold Post title:string body:text
doido generate auth:scaffold Post title:string --api
```

- Ensures auth is installed.
- Scaffolds the resource with `#[before_action(require_user)]` on all actions.
- Associates records with `user_id` when a `references` field is omitted (adds
  `user:references` automatically).
- Injects `resources!(posts, PostsController);` into `config/routes.rs`.

### `doido new --auth`

New apps opt in at creation — this is the primary bootstrap that **adds**
`doido-auth` to `Cargo.toml` and immediately runs `auth:install`:

```sh
doido new myapp --database=sqlite --auth
```

Equivalent to `doido new` with `doido-auth` in the template `Cargo.toml`, followed
by `doido generate auth:install`.

## Custom strategies

Third-party auth backends (LDAP, SAML, magic link) implement `AuthStrategy`:

```rust
pub trait AuthStrategy: Send + Sync {
    fn name(&self) -> &str;
    async fn authenticate(&self, parts: &Parts, db: &DatabaseConnection)
        -> Result<Option<AuthIdentity>>;
}

doido_auth::register_strategy("ldap", Arc::new(LdapStrategy::new(config)));
```

Enable in config: `strategies: [cookie, ldap]`.

## TDD surface

- `user_test` — `AuthUser` trait object safety, default `find_by_*` helpers.
- `session_test` — sign-in stores session, sign-out clears, tampered cookie rejected.
- `jwt_test` — issue/verify/refresh/expire/revoke; wrong secret fails.
- `oauth_test` — OAuth2 authorization URL + callback token exchange (mock HTTP).
- `extractors_test` — `CurrentUser` 401 when absent; `MaybeUser` returns None;
  strategy priority order.
- `routes_test` — POST sign_in creates session; registration validates email uniqueness;
  password reset flow end-to-end (with `doido_mailer::TestDeliverer`).
- `two_factor_test` — enroll/confirm/challenge/backup codes (feature `auth-2fa`).
- `config_test` — YAML parse, missing secret errors, strategy toggle.
- `generators_test` — `auth:install` emits expected files; route injection idempotent;
  generators absent from CLI list when `doido-auth` not in `Cargo.toml`.
- `cli_discovery_test` (in `doido-generators`) — `project_has_doido_auth` parses
  `Cargo.toml`; auth generators merged only when true; `doido generate` help output
  excludes auth section without the dep.

## E2E scenario

`doido-generators/tests/e2e/scenarios/auth_install.rs` (backlog US-113):

1. `doido new blog --database=sqlite --auth` (adds `doido-auth` + runs install)
   — or `cargo add doido-auth && doido generate auth:install --api`
2. `doido db create && doido db migrate`
4. Boot server; POST `/users/sign_up` → POST `/users/sign_in` → GET protected
   route with session cookie → `401` without cookie.

## Deferred (backlog)

- `#[auth_user]` proc-macro derive (manual `AuthUser` impl is v1).
- Magic-link / passwordless email sign-in.
- SAML / WebAuthn strategies.
- Multi-tenant / account scoping (`AuthUser` belongs to `Account`).
- Doorkeeper-style OAuth *provider* (issue tokens *to* third-party apps).
