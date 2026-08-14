+++
title = "Auth"
description = "Autenticación unificada — trait AuthUser, estrategias cookie/JWT/OAuth, extractors, 2FA y generadores de auth."
weight = 13
+++

> **Especificación de diseño:** [`docs/16-auth.md`](https://github.com/doido-rs/doido/blob/master/docs/16-auth.md).
> Esta guía documenta la API tal como está implementada en `doido-auth`.

**Análogo en Rails: Devise + OmniAuth + JWT.** `doido-auth` ofrece la trait
genérica `AuthUser` ligada a tu modelo SeaORM, **estrategias** enchufables (sesión
por cookie, JWT bearer, OAuth2), **2FA** opcional (TOTP), **extractors** axum y
**generadores** que generan controladores de sesión, registro, contraseña y OAuth.
El hash de contraseña reutiliza `doido_model::password::HasSecurePassword`.

## Controladores integrados y módulos

`doido new --auth` / `auth:install` generan un modelo `User`, una migración, un bloque de
config `auth:` y un `auth_routes!(User)` simple — pero **no se copia ningún controlador ni
vista en tu app**. La autenticación corre sobre los controladores **integrados** de
doido-auth y renderiza HTML desde vistas provistas por el framework y sobreescribibles. Para
personalizarlos, *extráelos* (el análogo de `devise:controllers` + `devise:views`):

```bash
cargo doido generate auth:controllers          # controladores + vistas (+ --api / --views-only)
```

Los **módulos estilo Devise** seleccionan qué funciones están activas, en `config/<env>.yml`:

```yaml
auth:
  modules: [database_authenticatable, registerable, recoverable, rememberable, validatable]
```

`auth:install --modules=a,b,c` emite las columnas de migración de cada módulo y restringe las
rutas a los grupos seleccionados. Disponibles: `database_authenticatable`, `registerable`,
`recoverable`, `rememberable`, `trackable`, `timeoutable`, `validatable`, `confirmable`,
`lockable`, `omniauthable`, `two_factor_authenticatable`.

## De un vistazo

```rust
use doido::auth::{
    init, auth_layer, AuthUser, CurrentUser, MaybeUser, RequireAuth, AuthToken,
    mount, JwtStrategy, register_strategy,
};
```

Activa el crate mediante el meta crate `doido`:

```toml
[dependencies]
doido = { version = "0.0.9", features = ["auth"] }
```

## Primeros pasos

La vía más rápida es una app nueva con auth incluido:

```bash
doido new blog --database=sqlite --auth
cd blog
cargo doido db create && cargo doido db migrate
cargo doido server
```

En una app existente, añade la dependencia e instala:

```bash
cargo add doido --features auth   # o añade doido-auth directamente
cargo doido generate auth:install
cargo doido db migrate
```

`auth:install` genera la migración y modelo User, controladores de auth, vistas HTML
(o solo JSON con `--api`), un bloque `auth:` en la config e inyecta rutas de sign-in,
sign-up, contraseña y OAuth en `config/routes.rs`. **No** modifica `Cargo.toml` — la
dependencia debe estar ya presente.

## La trait `AuthUser`

Tu modelo User SeaORM implementa `AuthUser` — el contrato genérico del sujeto
autenticado:

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

Combínalo con `HasSecurePassword` para verificar credenciales. La v1 requiere impl
manual; un derive `#[auth_user]` está planificado para una release futura.

## Configuración

La sección `auth` de `config/<env>.yml` controla estrategias, JWT, proveedores OAuth,
2FA y rutas al estilo Devise:

```yaml
auth:
  user_model: User
  strategies:
    - cookie                    # cookie de sesión (predeterminado)
    - jwt                       # Authorization: Bearer <token>
  jwt:
    secret: "${JWT_SECRET}"
    access_ttl: 900              # 15 minutos
    refresh_ttl: 604800          # 7 días
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
    enabled: false               # requiere feature auth-2fa
    issuer: MyApp
  routes:
    prefix: /users
    sign_in: sign_in
    sign_out: sign_out
    sign_up: sign_up
    password_reset: password
```

Los secretos vienen de `config/credentials.yml.enc` o variables de entorno. Activa JWT
en `strategies` solo cuando `auth.jwt.secret` esté definido.

## Secuencia de arranque

Inicializa auth tras el pool de base de datos y antes de servir peticiones:

```rust
use doido::auth::{init, auth_layer};

#[tokio::main]
async fn main() {
    let config = /* carga config */;
    let db = /* conecta pool */;

    init(db.clone(), &config.auth).await.expect("auth init");

    let app = config::routes::router()
        .layer(axum::middleware::from_fn(auth_layer));
    // …serve…
}
```

`init` registra estrategias habilitadas, carga configs OAuth y almacena el
`AuthState` global. `auth_layer` consulta estrategias en el orden de la config y
guarda la primera `AuthIdentity` resuelta en las extensiones de la petición.

## Estrategias

| Estrategia | Nombre en config | Cómo se resuelve la identidad |
|------------|------------------|-------------------------------|
| Cookie / sesión | `cookie` | Cookie de sesión cifrada con `user_id` |
| JWT bearer | `jwt` | Cabecera `Authorization: Bearer` |
| Personalizada | cualquier nombre registrado | Tu impl de `AuthStrategy` |

La sesión por cookie se integra con `doido_controller::session`. JWT emite pares
access + refresh mediante `JwtStrategy::issue_tokens`. Usa ambos para clientes HTML
(sesión) y API (bearer):

```rust
use doido::auth::{JwtStrategy, TokenPair};

let jwt = JwtStrategy::new(config.jwt.clone())?;
let tokens: TokenPair = jwt.issue_tokens(&serde_json::json!(user.id()))?;
```

Los extractors consultan estrategias en el orden de la config; gana la primera coincidencia.

## Extractors axum

| Extractor | Comportamiento | Estado HTTP al fallar |
|-----------|----------------|----------------------|
| `CurrentUser<U>` | Exige usuario autenticado, cargado del DB | `401 Unauthorized` |
| `MaybeUser<U>` | `Option<U>` — nunca falla | — |
| `RequireAuth` | Garantiza identidad sin cargar el modelo completo | `401 Unauthorized` |
| `AuthToken` | Cadena cruda del bearer token | `401` si falta |

```rust
use doido::auth::extractors::{CurrentUser, MaybeUser};
use doido::controller::axum::Json;

async fn profile(CurrentUser(user): CurrentUser<User>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "email": user.email() }))
}

async fn home(MaybeUser(user): MaybeUser<User>) -> String {
    match user.0 {
        Some(u) => format!("Bienvenido de nuevo, {}", u.email()),
        None => "Invitado".into(),
    }
}
```

Dentro de acciones `#[controller]`, llama a
`doido::auth::current_user::<User>(&ctx.parts())` o usa los extractors como
parámetros del handler.

## Rutas predefinidas

`auth:install` inyecta rutas explícitas en `config/routes.rs`:

| Método | Ruta | Acción del controlador |
|--------|------|------------------------|
| GET | `/users/sign_in` | `SessionsController::new` (HTML) |
| POST | `/users/sign_in` | `SessionsController::create` |
| DELETE | `/users/sign_out` | `SessionsController::destroy` |
| GET | `/users/sign_up` | `RegistrationsController::new` (HTML) |
| POST | `/users/sign_up` | `RegistrationsController::create` |
| POST | `/users/password` | `PasswordsController::create` |
| PATCH | `/users/password` | `PasswordsController::update` |
| GET | `/auth/{provider}` | `OauthController::authorize` |
| GET | `/auth/{provider}/callback` | `OauthController::callback` |

Para montaje programático sin controladores generados, usa `mount`:

```rust
use doido::auth::mount;

// `create` persiste un usuario recién registrado (email + contraseña en texto):
let auth_router = mount::<User, _>(|db, email, password| {
    Box::pin(async move {
        User::create(&db, email, password).await // helper específico de tu app
    })
});
```

Las rutas respetan `auth.routes` en la config (`prefix`, `sign_in`, etc.).

## OAuth

Los proveedores implementan la trait `OAuthProvider`. Las entradas con
`type: oauth2` en la config se convierten en instancias de `OAuth2Provider` en el
boot; los proveedores personalizados se registran en runtime:

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

let provider = get_provider("idp").expect("idp configurado");
let url = provider.authorize_url("random-state")?;
let tokens = provider.exchange_code("auth-code")?;
```

Las rutas de callback forman parte del `OauthController` generado. Las entradas
OAuth 1.0a en la config se reconocen, pero requieren una impl personalizada de
`OAuthProvider`.

## Autenticación de dos factores (feature `auth-2fa`)

Activa la feature y define `auth.two_factor.enabled: true`:

```toml
doido = { version = "0.0.9", features = ["auth", "auth-2fa"] }
```

```rust
use doido::auth::{enroll_two_factor, verify_two_factor_code, TwoFactorEnrollment};

let TwoFactorEnrollment { secret, otpauth_uri } = enroll_two_factor("user@example.com", "MyApp")?;
let ok = verify_two_factor_code(&secret, "123456")?;
```

`auth:install --two-factor` añade columnas `two_factor_secret` y
`two_factor_enabled` más controlador/vistas de 2FA. El almacenamiento de códigos de
respaldo está planificado para una release futura.

## Generadores

Los generadores de auth viven dentro de `doido-auth` y aparecen en `cargo doido generate`
**solo cuando** `doido-auth` está en el `Cargo.toml` del proyecto:

| Generador | Genera |
|-----------|--------|
| `auth:install` | Migración + modelo User, controladores de auth, vistas, config, rutas |
| `auth:controller <Name> actions…` | Controlador con `CurrentUser` / guards de auth |
| `auth:scaffold <Name> fields…` | Scaffold con auth y ownership por `user_id` |

```bash
cargo doido generate auth:install
cargo doido generate auth:install --api          # respuestas JSON, sin vistas HTML
cargo doido generate auth:install --two-factor     # columnas 2FA + controladores
cargo doido generate auth:controller Dashboard index show
cargo doido generate auth:scaffold Post title:string body:text
```

Sin `doido-auth` en `Cargo.toml`, los generadores de auth no aparecen en la lista y el
dispatch devuelve error apuntando a `doido new --auth` o `cargo add`.

## Estrategias personalizadas

Backends de terceros (LDAP, SAML, magic link) implementan `AuthStrategy`:

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
        Ok(None)
    }
}

register_strategy("ldap", Arc::new(LdapStrategy { /* … */ }));
```

Activa en config: `strategies: [cookie, ldap]`.

## Pruebas

`doido_auth::testing` proporciona fakes en memoria y helpers que serializan el estado
global de auth entre pruebas:

```rust
use doido_auth::testing::{AuthTestGuard, TestUser, seed_user, sign_in_request};

let _guard = AuthTestGuard::new();
seed_user(&db, "alice@example.com", "secret").await?;
let response = sign_in_request(&app, "alice@example.com", "secret").await?;
assert_eq!(response.status(), StatusCode::OK);
```

## Spec vs. implementación

> La spec describe la macro `auth_routes!(User)` con opciones `only:`/`skip:` al
> estilo Devise. **La v1 usa rutas explícitas** inyectadas por `auth:install` (o
> `routes::mount` para apps programáticas). Tablas de rotación de refresh token,
> intercambio OAuth 1.0a y el derive `#[auth_user]` están aplazados.

## Ver también

- [Middleware & sesiones](@/docs/reference/middleware.md) — el session store sobre el que auth se apoya.
- [Models](@/docs/reference/models.md) — `HasSecurePassword` para digests de contraseña.
- [Generadores & CLI](@/docs/reference/generators.md) — generadores de auth y `doido new --auth`.
- [Cable](@/docs/reference/cable.md) — autorizar conexiones WebSocket.
