+++
title = "Construindo um blog"
description = "Construa um blog renderizado no servidor com área de administração do autor e comentários."
weight = 2
+++

Este tutorial constrói um blog pequeno, porém completo, sobre a base do [Primeiros
passos](@/docs/tutorials/getting-started.md). Ao final você terá:

- uma página inicial **pública** que lista os posts publicados e uma página para ler um post,
- **comentários** que qualquer leitor pode deixar em um post,
- uma **área de administração do autor** — uma seção `/admin`, protegida pelo
  [`doido-auth`](@/docs/reference/auth.md), onde o autor escreve e publica os posts.

É um app HTML puro (sem API), e cada passo usa a implementação mais básica que funciona, para
que você enxergue as peças com clareza.

Este é o mapa de rotas que vamos alcançar:

| Método | Caminho | Quem | Objetivo |
|--------|---------|------|----------|
| GET | `/` | todos | listar posts publicados |
| GET | `/posts/:id` | todos | ler um post + seus comentários |
| POST | `/posts/:post_id/comments` | todos | deixar um comentário |
| GET/POST/… | `/admin/posts…` | autor autenticado | gerenciar posts |
| GET/POST | `/users/sign_in`, `/users/sign_up` | autor | autenticação (gerada pelo `--auth`) |

## Criar o app

Gere uma nova aplicação já com autenticação e configure o banco de dados:

```bash
# --auth adiciona o doido-auth e roda o auth:install (model User, controllers de sign-in/up + rotas)
doido new blog --database=sqlite --auth
cd blog

cargo doido db create
cargo doido db migrate      # cria a tabela users

cargo doido server          # http://0.0.0.0:3000 — sign-in/up já funcionam
```

O `--auth` fornece um model `User`, um `SessionsController` e um `RegistrationsController`, além
das rotas de sign-in / sign-up / sign-out sob `/users`. Vamos nos apoiar nisso para proteger a
área de administração. Veja a [referência de Auth](@/docs/reference/auth.md) para o panorama
completo.

## O model Post

Um post tem título, corpo, um indicador de publicação e um autor (o `User` autenticado). Gere o
model e sua migration:

```bash
cargo doido generate model Post \
  title:string:not_null \
  body:text:not_null \
  published:boolean:not_null \
  user:references
cargo doido db migrate
```

O `user:references` adiciona uma coluna de chave estrangeira `user_id` (um `i64` não nulo). O
gerador escreve `app/models/post.rs` — uma entidade [sea-orm](@/docs/reference/models.md)
comum. Adicione as relações para navegar do post aos seus comentários e ao seu autor:

```rust
// app/models/post.rs
use doido::model::sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub title: String,
    pub body: String,
    pub published: bool,
    pub user_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::comment::Entity")]
    Comments,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}
```

Já que estamos aqui, adicione uma pequena validação para rejeitar posts vazios. O trait
[`Validate`](@/docs/reference/models.md) do Doido acumula os erros:

```rust
use doido::model::validation::{Validate, Errors};

impl Validate for Model {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", &self.body, Some(10), None); // pelo menos 10 caracteres
        e
    }
}
```

O gerador também deixou um esqueleto em `tests/post_model_test.rs` — vamos preenchê-lo em
[Testes](#testes).

## O model Comment

Um comentário pertence a um post e carrega o nome do leitor e a mensagem. Não é preciso login
para comentar, então guardamos apenas um nome em texto livre:

```bash
cargo doido generate model Comment \
  post:references \
  author_name:string:not_null \
  body:text:not_null
cargo doido db migrate
```

Adicione a relação inversa de volta ao `Post`:

```rust
// app/models/comment.rs
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::post::Entity",
        from = "Column::PostId",
        to = "super::post::Column::Id"
    )]
    Post,
}
```

## Rotas

Abra `config/routes.rs` e descreva o app. Mantenha as rotas de auth que o `--auth` já injetou;
acrescente as rotas públicas, a rota de comentário e o namespace admin:

```rust
// config/routes.rs
use crate::controllers::{CommentsController, PostsController};
use crate::controllers::admin::PostsController as AdminPostsController;
use doido::controller::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        root!(PostsController::index);                       // GET /
        resources!(posts, PostsController, only: [index, show]);
        post!("/posts/:post_id/comments", CommentsController::create);

        namespace!(admin, {                                  // prefixo de caminho + helper "admin"
            resources!(posts, AdminPostsController);         // /admin/posts … (as 7 rotas)
        });

        // As rotas /users de sign-in, sign-up e sign-out foram injetadas pelo --auth — mantenha-as.
    }
}
```

O `namespace!(admin, …)` prefixa tanto a URL (`/admin/posts`) quanto os helpers de caminho
gerados (`admin_posts_path()`), de modo que nunca colidem com o `posts_path()` público. Veja
[Controllers & rotas](@/docs/reference/controllers.md) para a DSL completa.

## O blog público

O controller público lê do banco de dados via `ctx.db()` e renderiza templates Tera. Crie
`app/controllers/posts_controller.rs`:

```rust
// app/controllers/posts_controller.rs
use crate::models::{comment, post};
use doido::controller::{controller, Context, Response};
use doido::model::serialization::as_json;
use doido::model::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;

pub struct PostsController;

#[controller]
impl PostsController {
    pub async fn index(ctx: Context) -> Response {
        let posts = post::Entity::find()
            .filter(post::Column::Published.eq(true))
            .all(ctx.db())
            .await
            .unwrap_or_default();

        ctx.render("posts/index", json!({ "posts": as_json(&posts) }))
    }

    pub async fn show(ctx: Context) -> Response {
        let Some(id) = ctx.param("id").and_then(|s| s.parse::<i64>().ok()) else {
            return ctx.status(404);
        };

        let Ok(Some(post)) = post::Entity::find_by_id(id).one(ctx.db()).await else {
            return ctx.status(404);
        };

        let comments = comment::Entity::find()
            .filter(comment::Column::PostId.eq(post.id))
            .all(ctx.db())
            .await
            .unwrap_or_default();

        ctx.render(
            "posts/show",
            json!({ "post": as_json(&post), "comments": as_json(&comments) }),
        )
    }
}
```

Registre-o em `app/controllers/mod.rs` (o gerador mantém essa lista; adicione os módulos se
ainda não estiverem lá):

```rust
// app/controllers/mod.rs
pub mod admin;
pub mod comments_controller;
pub mod posts_controller;

pub use comments_controller::CommentsController;
pub use posts_controller::PostsController;
```

### Views

Os templates ficam em `app/views/<controller>/<action>.html.tera` e são renderizados como
**fragmentos** envolvidos por `app/views/layouts/application.html.tera`, que injeta o conteúdo
com `{{ content_for_layout }}` — não há `{% extends %}`. O JSON que você passa para `ctx.render`
vira o contexto do template.

```html
{# app/views/posts/index.html.tera #}
<h1>Blog</h1>
{% for post in posts %}
  <article>
    <h2><a href="/posts/{{ post.id }}">{{ post.title }}</a></h2>
  </article>
{% endfor %}
```

```html
{# app/views/posts/show.html.tera #}
<article>
  <h1>{{ post.title }}</h1>
  <p>{{ post.body }}</p>
</article>

<section>
  <h2>Comentários</h2>
  {% for comment in comments %}
    <p><strong>{{ comment.author_name }}</strong>: {{ comment.body }}</p>
  {% endfor %}

  <form method="post" action="/posts/{{ post.id }}/comments">
    <input type="text" name="author_name" placeholder="Seu nome" required>
    <textarea name="body" placeholder="Seu comentário" required></textarea>
    <button type="submit">Comentar</button>
  </form>
</section>
```

## Comentários

O formulário de comentário acima envia para `CommentsController::create`. Ele lê o corpo do
formulário em uma struct tipada, insere uma linha e redireciona de volta ao post. Crie
`app/controllers/comments_controller.rs`:

```rust
// app/controllers/comments_controller.rs
use crate::models::comment;
use doido::controller::{controller, Context, Response};
use doido::model::{ActiveModelTrait, Set};
use serde::Deserialize;

#[derive(Deserialize)]
struct NewComment {
    author_name: String,
    body: String,
}

pub struct CommentsController;

#[controller]
impl CommentsController {
    pub async fn create(ctx: Context) -> Response {
        let Some(post_id) = ctx.param("post_id").and_then(|s| s.parse::<i64>().ok()) else {
            return ctx.status(404);
        };

        let Ok(form) = ctx.form::<NewComment>().await else {
            return ctx.redirect_to(format!("/posts/{post_id}"));
        };

        let comment = comment::ActiveModel {
            post_id: Set(post_id),
            author_name: Set(form.author_name),
            body: Set(form.body),
            ..Default::default()
        };
        let _ = comment.insert(ctx.db()).await;

        ctx.redirect_to(format!("/posts/{post_id}"))
    }
}
```

## A área de administração do autor

A área de administração é um controller comum colocado em um módulo `admin` e protegido por um
filtro `before_action`. Quando o autor faz login, o `doido-auth` guarda o id dele na sessão; o
filtro lê esse id de volta e, se ninguém estiver autenticado, interrompe a requisição e
redireciona para a página de login. (Você também poderia receber o extractor `CurrentUser<User>`
como argumento da action, como faz o `auth:scaffold` gerado — veja a
[referência de Auth](@/docs/reference/auth.md); aqui deixamos explícito.)

Crie `app/controllers/admin/mod.rs`:

```rust
// app/controllers/admin/mod.rs
pub mod posts_controller;
pub use posts_controller::PostsController;
```

Depois `app/controllers/admin/posts_controller.rs`:

```rust
// app/controllers/admin/posts_controller.rs
use crate::models::post;
use doido::controller::{controller, Context, Response};
use doido::model::serialization::as_json;
use doido::model::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::json;

// Interrompe e redireciona para o login se ninguém estiver autenticado. O login guarda
// o id do usuário na sessão sob "user_id" (veja o doido-auth).
async fn require_login(ctx: &mut Context) -> Result<(), Response> {
    if ctx.session().get::<i64>("user_id").is_none() {
        return Err(ctx.redirect_to("/users/sign_in"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct PostForm {
    title: String,
    body: String,
    published: Option<String>, // um checkbox desmarcado simplesmente não vem
}

pub struct PostsController;

#[controller]
impl PostsController {
    #[before_action(require_login)]
    pub async fn index(mut ctx: Context) -> Response {
        let author_id = ctx.session().get::<i64>("user_id").unwrap();
        let posts = post::Entity::find()
            .filter(post::Column::UserId.eq(author_id))
            .all(ctx.db())
            .await
            .unwrap_or_default();

        ctx.render("admin/posts/index", json!({ "posts": as_json(&posts) }))
    }

    #[before_action(require_login)]
    pub async fn new(ctx: Context) -> Response {
        ctx.render("admin/posts/new", json!({}))
    }

    #[before_action(require_login)]
    pub async fn create(mut ctx: Context) -> Response {
        let author_id = ctx.session().get::<i64>("user_id").unwrap();
        let Ok(form) = ctx.form::<PostForm>().await else {
            return ctx.redirect_to("/admin/posts/new");
        };

        let post = post::ActiveModel {
            title: Set(form.title),
            body: Set(form.body),
            published: Set(form.published.is_some()),
            user_id: Set(author_id),
            ..Default::default()
        };
        let _ = post.insert(ctx.db()).await;

        ctx.redirect_to("/admin/posts")
    }

    // edit / update / destroy seguem o mesmo formato: ler author_id da sessão, carregar o
    // post, checar que ele pertence a esse autor, e então renderizar, salvar ou apagar.
}
```

Registre o módulo em `app/controllers/mod.rs` (já adicionamos `pub mod admin;` acima).

Dois templates de administração mínimos:

```html
{# app/views/admin/posts/index.html.tera #}
<h1>Seus posts</h1>
<a href="/admin/posts/new">Escrever um post</a>
<ul>
  {% for post in posts %}
    <li>
      {{ post.title }}
      {% if post.published %}(publicado){% else %}(rascunho){% endif %}
    </li>
  {% endfor %}
</ul>
```

```html
{# app/views/admin/posts/new.html.tera #}
<h1>Novo post</h1>
<form method="post" action="/admin/posts">
  <input type="text" name="title" placeholder="Título" required>
  <textarea name="body" placeholder="Escreva seu post…" required></textarea>
  <label><input type="checkbox" name="published" value="1"> Publicar agora</label>
  <button type="submit">Salvar</button>
</form>
```

## Rodando

```bash
cargo doido server
```

Agora percorra todo o fluxo:

1. Acesse `/users/sign_up` e registre a conta do autor.
2. Vá em `/admin/posts`, escreva um post e marque **Publicar agora**.
3. Abra `/` — o post publicado aparece. Clique para ir a `/posts/:id`.
4. Deixe um comentário; ele aparece abaixo do post.

Ao sair (`DELETE /users/sign_out`) e voltar a `/admin/posts`, você é levado de volta à página de
login — é o `require_login` fazendo seu trabalho.

## Testes

Apps Doido são testados com funções `#[tokio::test]` simples. Três tipos de teste cobrem este
blog: um teste de **model**, um teste de **requisição** e um teste de **auth**. Rode todos com
`cargo test` (ou um único com `cargo test <nome>`).

### Testes de model

O `TestDb` sobe um banco SQLite em memória e isolado. Crie a tabela, insira uma linha e verifique
que ela persiste. Este é o esqueleto que o gerador deixou em `tests/post_model_test.rs`:

```rust
// tests/post_model_test.rs
#[path = "../app/models/mod.rs"]
mod models;

use doido::model::sea_orm::{ConnectionTrait, Schema};
use doido::model::{ActiveModelTrait, EntityTrait, Set, TestDb};
use models::post;

#[tokio::test]
async fn creates_and_finds_a_post() {
    let db = TestDb::new().await.unwrap();

    // Constrói a tabela posts a partir da definição da entidade.
    let backend = db.conn().get_database_backend();
    let stmt = Schema::new(backend).create_table_from_entity(post::Entity);
    db.conn().execute(backend.build(&stmt)).await.unwrap();

    let created = post::ActiveModel {
        title: Set("Olá".into()),
        body: Set("Meu primeiro post, longo o bastante.".into()),
        published: Set(true),
        user_id: Set(1),
        ..Default::default()
    }
    .insert(db.conn())
    .await
    .unwrap();

    let found = post::Entity::find_by_id(created.id)
        .one(db.conn())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.title, "Olá");
    assert!(found.published);
}
```

Validações são lógica pura, então não precisam de banco algum:

```rust
#[test]
fn rejects_a_blank_post() {
    use doido::model::validation::Validate;

    let post = models::post::Model {
        id: 0,
        title: String::new(),      // ausente
        body: "curto".into(),      // < 10 caracteres
        published: false,
        user_id: 1,
    };

    assert!(!post.is_valid());
    // post.validate().full_messages() lista os erros legíveis.
}
```

### Testes de requisição

Testes de requisição montam o router real e o acionam com um cliente em processo — sem servidor
ativo. Isso espelha o teste de requisição que o gerador `scaffold` do Doido produz: inclua os
módulos do app com `#[path]`, instale um pool em memória uma vez no `setup()` e verifique os
códigos de status.

```rust
// tests/posts_request_test.rs
#[path = "../app/controllers/mod.rs"]
mod controllers;
#[path = "../app/models/mod.rs"]
mod models;
#[path = "../config/routes.rs"]
mod routes;

use doido::controller::axum;
use doido::model::sea_orm::{ConnectionTrait, Database, Schema};
use models::post;
use tower::ServiceExt; // para `oneshot`

async fn setup() {
    if doido::model::pool::try_pool().is_none() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let _ = doido::model::pool::set_pool(db);
    }
    let db = doido::model::pool::pool();
    let backend = db.get_database_backend();
    let stmt = Schema::new(backend).create_table_from_entity(post::Entity);
    let _ = db.execute(backend.build(stmt.if_not_exists())).await;
}

#[tokio::test]
async fn index_is_public() {
    setup().await;

    let response = routes::router()
        .oneshot(
            axum::http::Request::get("/")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn admin_redirects_when_signed_out() {
    setup().await;

    let response = routes::router()
        .oneshot(
            axum::http::Request::get("/admin/posts")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // require_login interrompe e redireciona para a página de login.
    assert_eq!(response.status(), axum::http::StatusCode::FOUND);
}
```

Para casos simples, `doido_controller::testing::send(router, "GET", "/", "")` devolve status e
corpo em uma só chamada, em vez de montar a requisição manualmente.

### Testes de auth

Para exercitar uma requisição *autenticada*, o `doido-auth` traz um harness de testes em memória.
O `seed_user` cria um usuário e o `sign_in_request` devolve uma requisição já com a sessão:

```rust
// tests/admin_auth_test.rs
use doido_auth::testing::{seed_user, sign_in_request, AuthTestGuard};

#[tokio::test]
async fn author_can_reach_the_admin_area() {
    let _guard = AuthTestGuard::new();
    // …prepare o pool + tabelas como no teste de requisição…

    seed_user(pool, "author@example.com", "s3cret").await.unwrap();
    let response = sign_in_request(&app, "author@example.com", "s3cret").await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
```

Veja a [referência de Auth](@/docs/reference/auth.md#testing) para a lista completa de helpers.

## Próximos passos

- **[Models](@/docs/reference/models.md)** — associações, migrations, validações e factories.
- **[Controllers & rotas](@/docs/reference/controllers.md)** — filtros, strong parameters e a DSL `routes!`.
- **[Views](@/docs/reference/views.md)** — layouts, partials e view helpers.
- **[Auth](@/docs/reference/auth.md)** — sessões, JWT, OAuth, 2FA e os extractors.
- **[Geradores & CLI](@/docs/reference/generators.md)** — todos os geradores, incluindo o `scaffold` para CRUD completo em um comando.
