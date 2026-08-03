+++
title = "Auth"
description = "Autenticação unificada — trait AuthUser, estratégias cookie/JWT/OAuth, extractors, 2FA e geradores de auth."
weight = 13
+++

> **Especificação de design:** [`docs/16-auth.md`](https://github.com/doido-rs/doido/blob/master/docs/16-auth.md).
> Este guia documenta a API como implementada em `doido-auth`.

**Análogo no Rails: Devise + OmniAuth + JWT.** O `doido-auth` oferece a trait
genérica `AuthUser` ligada ao seu model SeaORM, **estratégias** plugáveis (sessão
por cookie, JWT bearer, OAuth2), **2FA** opcional (TOTP), **extractors** axum e
**geradores** que montam controllers de sessão, registro, senha e OAuth. O hash
de senha reutiliza `doido_model::password::HasSecurePassword`.

## Visão geral

```rust
use doido::auth::{
    init, auth_layer, AuthUser, CurrentUser, MaybeUser, RequireAuth, AuthToken,
    mount, JwtStrategy, register_strategy,
};
```

Ative o crate via o meta crate `doido`:

```toml
[dependencies]
doido = { version = "0.0.9", features = ["auth"] }
```

## Primeiros passos

O caminho mais rápido é uma app nova com auth incluso:

```bash
doido new blog --database=sqlite --auth
cd blog
cargo doido db create && cargo doido db migrate
cargo doido server
```

Em uma app existente, adicione a dependência e instale:

```bash
cargo add doido --features auth   # ou adicione doido-auth diretamente
cargo doido generate auth:install
cargo doido db migrate
```

O `auth:install` gera a migration e model de User, controllers de auth, views HTML
(ou só JSON com `--api`), um trecho `auth:` na config e injeta rotas de sign-in,
sign-up, senha e OAuth em `config/routes.rs`. **Não** altera o `Cargo.toml` — a
dependência precisa já estar presente.

## A trait `AuthUser`

Seu model User SeaORM implementa `AuthUser` — o contrato genérico do sujeito
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

Combine com `HasSecurePassword` para verificação de credenciais. A v1 exige impl
manual; um derive `#[auth_user]` está planejado para uma release futura.

## Configuração

A seção `auth` de `config/<env>.yml` controla estratégias, JWT, provedores OAuth,
2FA e caminhos de rota no estilo Devise:

```yaml
auth:
  user_model: User
  strategies:
    - cookie                    # cookie de sessão (padrão)
    - jwt                       # Authorization: Bearer <token>
  jwt:
    secret: "${JWT_SECRET}"
    access_ttl: 900              # 15 minutos
    refresh_ttl: 604800          # 7 dias
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
    enabled: false               # requer feature auth-2fa
    issuer: MyApp
  routes:
    prefix: /users
    sign_in: sign_in
    sign_out: sign_out
    sign_up: sign_up
    password_reset: password
```

Segredos vêm de `config/credentials.yml.enc` ou variáveis de ambiente. Ative JWT
em `strategies` somente quando `auth.jwt.secret` estiver definido.

## Sequência de boot

Inicialize o auth após o pool do banco e antes de servir requisições:

```rust
use doido::auth::{init, auth_layer};

#[tokio::main]
async fn main() {
    let config = /* carrega config */;
    let db = /* conecta pool */;

    init(db.clone(), &config.auth).await.expect("auth init");

    let app = config::routes::router()
        .layer(axum::middleware::from_fn(auth_layer));
    // …serve…
}
```

O `init` registra estratégias habilitadas, carrega configs OAuth e armazena o
`AuthState` global. O `auth_layer` consulta estratégias na ordem da config e
grava a primeira `AuthIdentity` resolvida nas extensions da requisição.

## Estratégias

| Estratégia | Nome na config | Como a identidade é resolvida |
|------------|----------------|-------------------------------|
| Cookie / sessão | `cookie` | Cookie de sessão criptografado com `user_id` |
| JWT bearer | `jwt` | Header `Authorization: Bearer` |
| Customizada | qualquer nome registrado | Sua impl de `AuthStrategy` |

A sessão por cookie integra com `doido_controller::session`. O JWT emite pares de
access + refresh via `JwtStrategy::issue_tokens`. Use ambos para clientes HTML
(sessão) e API (bearer):

```rust
use doido::auth::{JwtStrategy, TokenPair};

let jwt = JwtStrategy::new(config.jwt.clone())?;
let tokens: TokenPair = jwt.issue_tokens(&serde_json::json!(user.id()))?;
```

Os extractors consultam estratégias na ordem da config; a primeira correspondência
vence.

## Extractors axum

| Extractor | Comportamento | Status HTTP em falha |
|-----------|---------------|----------------------|
| `CurrentUser<U>` | Exige usuário autenticado, carregado do DB | `401 Unauthorized` |
| `MaybeUser<U>` | `Option<U>` — nunca falha | — |
| `RequireAuth` | Garante identidade sem carregar o model completo | `401 Unauthorized` |
| `AuthToken` | String crua do bearer token | `401` se ausente |

```rust
use doido::auth::extractors::{CurrentUser, MaybeUser};
use doido::controller::axum::Json;

async fn profile(CurrentUser(user): CurrentUser<User>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "email": user.email() }))
}

async fn home(MaybeUser(user): MaybeUser<User>) -> String {
    match user.0 {
        Some(u) => format!("Bem-vindo de volta, {}", u.email()),
        None => "Visitante".into(),
    }
}
```

Dentro de actions `#[controller]`, chame
`doido::auth::current_user::<User>(&ctx.parts())` ou use os extractors como
parâmetros do handler.

## Rotas pré-montadas

O `auth:install` injeta rotas explícitas em `config/routes.rs`:

| Método | Caminho | Action do controller |
|--------|---------|----------------------|
| GET | `/users/sign_in` | `SessionsController::new` (HTML) |
| POST | `/users/sign_in` | `SessionsController::create` |
| DELETE | `/users/sign_out` | `SessionsController::destroy` |
| GET | `/users/sign_up` | `RegistrationsController::new` (HTML) |
| POST | `/users/sign_up` | `RegistrationsController::create` |
| POST | `/users/password` | `PasswordsController::create` |
| PATCH | `/users/password` | `PasswordsController::update` |
| GET | `/auth/{provider}` | `OauthController::authorize` |
| GET | `/auth/{provider}/callback` | `OauthController::callback` |

Para montagem programática sem controllers gerados, use `mount`:

```rust
use doido::auth::mount;

// `create` persiste um usuário recém-registrado (email + senha em texto):
let auth_router = mount::<User, _>(|db, email, password| {
    Box::pin(async move {
        User::create(&db, email, password).await // helper específico da sua app
    })
});
```

Os caminhos respeitam `auth.routes` na config (`prefix`, `sign_in`, etc.).

## OAuth

Provedores implementam a trait `OAuthProvider`. Entradas com `type: oauth2` na
config viram instâncias de `OAuth2Provider` no boot; provedores customizados
registram em runtime:

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

As rotas de callback fazem parte do `OauthController` gerado. Entradas OAuth 1.0a
na config são reconhecidas, mas exigem uma impl customizada de `OAuthProvider`.

## Autenticação de dois fatores (feature `auth-2fa`)

Ative a feature e defina `auth.two_factor.enabled: true`:

```toml
doido = { version = "0.0.9", features = ["auth", "auth-2fa"] }
```

```rust
use doido::auth::{enroll_two_factor, verify_two_factor_code, TwoFactorEnrollment};

let TwoFactorEnrollment { secret, otpauth_uri } = enroll_two_factor("user@example.com", "MyApp")?;
let ok = verify_two_factor_code(&secret, "123456")?;
```

O `auth:install --two-factor` adiciona colunas `two_factor_secret` e
`two_factor_enabled` mais controller/views de 2FA. Armazenamento de códigos de
backup está planejado para uma release futura.

## Geradores

Os geradores de auth ficam dentro de `doido-auth` e aparecem em `cargo doido generate`
**somente quando** `doido-auth` está no `Cargo.toml` do projeto:

| Gerador | Gera |
|---------|------|
| `auth:install` | Migration + model User, controllers de auth, views, config, rotas |
| `auth:controller <Name> actions…` | Controller com `CurrentUser` / guards de auth |
| `auth:scaffold <Name> fields…` | Scaffold com auth e ownership por `user_id` |

```bash
cargo doido generate auth:install
cargo doido generate auth:install --api          # respostas JSON, sem views HTML
cargo doido generate auth:install --two-factor   # colunas 2FA + controllers
cargo doido generate auth:controller Dashboard index show
cargo doido generate auth:scaffold Post title:string body:text
```

Sem `doido-auth` no `Cargo.toml`, os geradores de auth não aparecem na lista e o
dispatch retorna erro apontando para `doido new --auth` ou `cargo add`.

## Estratégias customizadas

Backends de terceiros (LDAP, SAML, magic link) implementam `AuthStrategy`:

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

Ative na config: `strategies: [cookie, ldap]`.

## Testes

`doido_auth::testing` fornece fakes em memória e helpers que serializam o estado
global de auth entre testes:

```rust
use doido_auth::testing::{AuthTestGuard, TestUser, seed_user, sign_in_request};

let _guard = AuthTestGuard::new();
seed_user(&db, "alice@example.com", "secret").await?;
let response = sign_in_request(&app, "alice@example.com", "secret").await?;
assert_eq!(response.status(), StatusCode::OK);
```

## Spec vs. implementação

> A spec descreve a macro `auth_routes!(User)` com opções `only:`/`skip:` no
> estilo Devise. **A v1 usa rotas explícitas** injetadas pelo `auth:install` (ou
> `routes::mount` para apps programáticas). Tabelas de rotação de refresh token,
> troca OAuth 1.0a e o derive `#[auth_user]` estão adiados.

## Veja também

- [Middleware & sessões](@/docs/reference/middleware.md) — o session store que o auth usa.
- [Models](@/docs/reference/models.md) — `HasSecurePassword` para digests de senha.
- [Geradores & CLI](@/docs/reference/generators.md) — geradores de auth e `doido new --auth`.
- [Cable](@/docs/reference/cable.md) — autorizar conexões WebSocket.
