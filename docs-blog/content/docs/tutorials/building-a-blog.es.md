+++
title = "Construyendo un blog"
description = "Construye un blog renderizado en el servidor con área de administración del autor y comentarios."
weight = 2
+++

Este tutorial construye un blog pequeño pero completo sobre la base de [Primeros
pasos](@/docs/tutorials/getting-started.md). Al final tendrás:

- una página de inicio **pública** que lista los posts publicados y una página para leer un post,
- **comentarios** que cualquier lector puede dejar en un post,
- un **área de administración del autor** — una sección `/admin`, protegida por
  [`doido-auth`](@/docs/reference/auth.md), donde el autor escribe y publica los posts.

Es una app HTML pura (sin API), y cada paso usa la implementación más básica que funciona, para
que veas las piezas con claridad.

Este es el mapa de rutas al que vamos a llegar:

| Método | Ruta | Quién | Propósito |
|--------|------|-------|-----------|
| GET | `/` | todos | listar posts publicados |
| GET | `/posts/:id` | todos | leer un post + sus comentarios |
| POST | `/posts/:post_id/comments` | todos | dejar un comentario |
| GET/POST/… | `/admin/posts…` | autor autenticado | gestionar posts |
| GET/POST | `/users/sign_in`, `/users/sign_up` | autor | autenticación (generada por `--auth`) |

## Crear la app

Genera una nueva aplicación con autenticación incluida y configura la base de datos:

```bash
# --auth agrega doido-auth y ejecuta auth:install (modelo User, controladores de sign-in/up + rutas)
doido new blog --database=sqlite --auth
cd blog

cargo doido db create
cargo doido db migrate      # crea la tabla users

cargo doido server          # http://0.0.0.0:3000 — sign-in/up ya funcionan
```

`--auth` te da un modelo `User`, un `SessionsController` y un `RegistrationsController`, además
de las rutas de sign-in / sign-up / sign-out bajo `/users`. Nos apoyaremos en ellas para proteger
el área de administración. Consulta la [referencia de Auth](@/docs/reference/auth.md) para el
panorama completo.

## El modelo Post

Un post tiene título, cuerpo, un indicador de publicación y un autor (el `User` autenticado).
Genera el modelo y su migración:

```bash
cargo doido generate model Post \
  title:string:not_null \
  body:text:not_null \
  published:boolean:not_null \
  user:references
cargo doido db migrate
```

`user:references` agrega una columna de clave foránea `user_id` (un `i64` no nulo). El generador
escribe `app/models/post.rs` — una entidad [sea-orm](@/docs/reference/models.md) normal. Agrega
las relaciones para navegar desde el post hacia sus comentarios y su autor:

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

Ya que estamos aquí, agrega una pequeña validación para rechazar posts vacíos. El trait
[`Validate`](@/docs/reference/models.md) de Doido acumula los errores:

```rust
use doido::model::validation::{Validate, Errors};

impl Validate for Model {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", &self.body, Some(10), None); // al menos 10 caracteres
        e
    }
}
```

El generador también dejó un esqueleto en `tests/post_model_test.rs` — lo completaremos en
[Pruebas](#pruebas).

## El modelo Comment

Un comentario pertenece a un post y lleva el nombre del lector y el mensaje. No hace falta login
para comentar, así que solo guardamos un nombre en texto libre:

```bash
cargo doido generate model Comment \
  post:references \
  author_name:string:not_null \
  body:text:not_null
cargo doido db migrate
```

Agrega la relación inversa de vuelta al `Post`:

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

## Rutas

Abre `config/routes.rs` y describe la app. Conserva las rutas de auth que `--auth` ya inyectó;
agrega las rutas públicas, la ruta de comentario y el namespace admin:

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

        namespace!(admin, {                                  // prefijo de ruta + helper "admin"
            resources!(posts, AdminPostsController);         // /admin/posts … (las 7 rutas)
        });

        // Las rutas /users de sign-in, sign-up y sign-out fueron inyectadas por --auth — consérvalas.
    }
}
```

`namespace!(admin, …)` antepone el prefijo tanto a la URL (`/admin/posts`) como a los helpers de
ruta generados (`admin_posts_path()`), de modo que nunca chocan con el `posts_path()` público.
Consulta [Controladores y rutas](@/docs/reference/controllers.md) para la DSL completa.

## El blog público

El controlador público lee de la base de datos vía `ctx.db()` y renderiza plantillas Tera. Crea
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

Regístralo en `app/controllers/mod.rs` (el generador mantiene esta lista; agrega los módulos si
aún no están):

```rust
// app/controllers/mod.rs
pub mod admin;
pub mod comments_controller;
pub mod posts_controller;

pub use comments_controller::CommentsController;
pub use posts_controller::PostsController;
```

### Vistas

Las plantillas viven en `app/views/<controller>/<action>.html.tera` y se renderizan como
**fragmentos** envueltos por `app/views/layouts/application.html.tera`, que inyecta el contenido
con `{{ content_for_layout }}` — no hay `{% extends %}`. El JSON que pasas a `ctx.render` se
convierte en el contexto de la plantilla.

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
  <h2>Comentarios</h2>
  {% for comment in comments %}
    <p><strong>{{ comment.author_name }}</strong>: {{ comment.body }}</p>
  {% endfor %}

  <form method="post" action="/posts/{{ post.id }}/comments">
    <input type="text" name="author_name" placeholder="Tu nombre" required>
    <textarea name="body" placeholder="Tu comentario" required></textarea>
    <button type="submit">Comentar</button>
  </form>
</section>
```

## Comentarios

El formulario de comentario anterior envía a `CommentsController::create`. Lee el cuerpo del
formulario en una struct tipada, inserta una fila y redirige de vuelta al post. Crea
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

## El área de administración del autor

El área de administración es un controlador normal ubicado en un módulo `admin` y protegido por un
filtro `before_action`. Cuando el autor inicia sesión, `doido-auth` guarda su id en la sesión; el
filtro lo lee de vuelta y, si nadie está autenticado, interrumpe la petición y redirige a la
página de inicio de sesión. (También podrías recibir el extractor `CurrentUser<User>` como
argumento de la action, como hace el `auth:scaffold` generado — consulta la
[referencia de Auth](@/docs/reference/auth.md); aquí lo dejamos explícito.)

Crea `app/controllers/admin/mod.rs`:

```rust
// app/controllers/admin/mod.rs
pub mod posts_controller;
pub use posts_controller::PostsController;
```

Luego `app/controllers/admin/posts_controller.rs`:

```rust
// app/controllers/admin/posts_controller.rs
use crate::models::post;
use doido::controller::{controller, Context, Response};
use doido::model::serialization::as_json;
use doido::model::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::json;

// Interrumpe y redirige al login si nadie está autenticado. El login guarda el id del
// usuario en la sesión bajo "user_id" (consulta doido-auth).
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
    published: Option<String>, // un checkbox sin marcar simplemente no llega
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

    // edit / update / destroy siguen el mismo formato: leer author_id de la sesión, cargar
    // el post, comprobar que pertenece a ese autor, y luego renderizar, guardar o borrar.
}
```

Registra el módulo en `app/controllers/mod.rs` (ya agregamos `pub mod admin;` arriba).

Dos plantillas de administración mínimas:

```html
{# app/views/admin/posts/index.html.tera #}
<h1>Tus posts</h1>
<a href="/admin/posts/new">Escribir un post</a>
<ul>
  {% for post in posts %}
    <li>
      {{ post.title }}
      {% if post.published %}(publicado){% else %}(borrador){% endif %}
    </li>
  {% endfor %}
</ul>
```

```html
{# app/views/admin/posts/new.html.tera #}
<h1>Nuevo post</h1>
<form method="post" action="/admin/posts">
  <input type="text" name="title" placeholder="Título" required>
  <textarea name="body" placeholder="Escribe tu post…" required></textarea>
  <label><input type="checkbox" name="published" value="1"> Publicar ahora</label>
  <button type="submit">Guardar</button>
</form>
```

## Ejecutarlo

```bash
cargo doido server
```

Ahora recorre todo el flujo:

1. Visita `/users/sign_up` y registra la cuenta del autor.
2. Ve a `/admin/posts`, escribe un post y marca **Publicar ahora**.
3. Abre `/` — el post publicado aparece. Haz clic para ir a `/posts/:id`.
4. Deja un comentario; aparece debajo del post.

Al cerrar sesión (`DELETE /users/sign_out`) y volver a `/admin/posts`, se te devuelve a la página
de inicio de sesión — es `require_login` haciendo su trabajo.

## Pruebas

Las apps Doido se prueban con funciones `#[tokio::test]` simples. Tres tipos de prueba cubren este
blog: una prueba de **modelo**, una prueba de **petición** y una prueba de **auth**. Ejecútalas
todas con `cargo test` (o una sola con `cargo test <nombre>`).

### Pruebas de modelo

`TestDb` levanta una base de datos SQLite en memoria y aislada. Crea la tabla, inserta una fila y
comprueba que persiste. Este es el esqueleto que el generador dejó en `tests/post_model_test.rs`:

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

    // Construye la tabla posts a partir de la definición de la entidad.
    let backend = db.conn().get_database_backend();
    let stmt = Schema::new(backend).create_table_from_entity(post::Entity);
    db.conn().execute(backend.build(&stmt)).await.unwrap();

    let created = post::ActiveModel {
        title: Set("Hola".into()),
        body: Set("Mi primer post, lo bastante largo.".into()),
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

    assert_eq!(found.title, "Hola");
    assert!(found.published);
}
```

Las validaciones son lógica pura, así que no necesitan base de datos alguna:

```rust
#[test]
fn rejects_a_blank_post() {
    use doido::model::validation::Validate;

    let post = models::post::Model {
        id: 0,
        title: String::new(),      // ausente
        body: "corto".into(),      // < 10 caracteres
        published: false,
        user_id: 1,
    };

    assert!(!post.is_valid());
    // post.validate().full_messages() lista los errores legibles.
}
```

### Pruebas de petición

Las pruebas de petición montan el router real y lo accionan con un cliente en proceso — sin
servidor activo. Esto refleja la prueba de petición que produce el generador `scaffold` de Doido:
incluye los módulos de la app con `#[path]`, instala un pool en memoria una vez en `setup()` y
verifica los códigos de estado.

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

    // require_login interrumpe y redirige a la página de inicio de sesión.
    assert_eq!(response.status(), axum::http::StatusCode::FOUND);
}
```

Para casos simples, `doido_controller::testing::send(router, "GET", "/", "")` devuelve el estado y
el cuerpo en una sola llamada, en lugar de construir la petición a mano.

### Pruebas de auth

Para ejercitar una petición *autenticada*, `doido-auth` incluye un harness de pruebas en memoria.
`seed_user` crea un usuario y `sign_in_request` devuelve una petición que ya lleva la sesión:

```rust
// tests/admin_auth_test.rs
use doido_auth::testing::{seed_user, sign_in_request, AuthTestGuard};

#[tokio::test]
async fn author_can_reach_the_admin_area() {
    let _guard = AuthTestGuard::new();
    // …prepara el pool + tablas como en la prueba de petición…

    seed_user(pool, "author@example.com", "s3cret").await.unwrap();
    let response = sign_in_request(&app, "author@example.com", "s3cret").await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
```

Consulta la [referencia de Auth](@/docs/reference/auth.md#testing) para la lista completa de
helpers.

## Próximos pasos

- **[Modelos](@/docs/reference/models.md)** — asociaciones, migraciones, validaciones y factories.
- **[Controladores y rutas](@/docs/reference/controllers.md)** — filtros, strong parameters y la DSL `routes!`.
- **[Vistas](@/docs/reference/views.md)** — layouts, partials y view helpers.
- **[Auth](@/docs/reference/auth.md)** — sesiones, JWT, OAuth, 2FA y los extractors.
- **[Generadores y CLI](@/docs/reference/generators.md)** — todos los generadores, incluido `scaffold` para CRUD completo en un comando.
