+++
title = "Construyendo un blog"
description = "Construye un blog renderizado en el servidor con un scaffold, autoría protegida por login y comentarios de lectores."
weight = 2
+++

Este tutorial construye un blog pequeño pero completo sobre la base de [Primeros
pasos](@/docs/tutorials/getting-started.es.md). Al final tendrás:

- una portada **pública** que lista los posts publicados y una página para leer uno,
- **comentarios** que cualquier lector puede dejar en un post,
- **autoría protegida por login** — escribir y publicar posts está protegido por
  [`doido-auth`](@/docs/reference/auth.es.md), y cada post pertenece a su autor.

Es una app HTML pura (sin API). Nos apoyamos en **generadores**: `scaffold` construye el recurso
Post entero en un comando, y `generate controller` nos da el endpoint de comentarios. Solo
editas a mano las *personalizaciones* — los generadores escriben (y conectan) los esqueletos.

> Cada comando y bloque de código de abajo es ejecutado por el propio e2e de release de Doido
> (`doido-generators/tests/e2e/scenarios/blog_tutorial.rs`), así que el tutorial se mantiene
> ejecutable. Ve el [estándar de tutoriales](@/docs/reference/generators.es.md).

Este es el mapa de rutas al que apuntamos:

| Método | Ruta | Quién | Propósito |
|--------|------|-------|-----------|
| GET | `/` | todos | lista los posts publicados |
| GET | `/posts/{id}` | todos | leer un post + sus comentarios |
| POST | `/posts/{post_id}/comments` | todos | dejar un comentario |
| GET/POST/… | `/posts/new`, `/posts`, `/posts/{id}/edit`… | autor con sesión | gestionar posts |
| GET/POST | `/users/sign_in`, `/users/sign_up` | autor | auth (generado por `--auth`) |

## Crear la app

Genera una nueva aplicación con autenticación incluida y prepara la base de datos:

```bash
# --auth añade doido-auth y ejecuta auth:install (modelo User, controladores de sign-in/up + rutas)
doido new blog --database=sqlite --auth
cd blog

cargo doido db create
cargo doido db migrate      # crea la tabla users

cargo doido server          # http://0.0.0.0:3000 — sign-in/up ya funcionan
```

`--auth` te da un modelo `User`, un `SessionsController` y un `RegistrationsController`, y rutas
de sign-in / sign-up / sign-out bajo `/users`. Nos apoyaremos en ellas para proteger la autoría.
Ve la [referencia de Auth](@/docs/reference/auth.es.md) para el panorama completo.

## Scaffold del recurso Post

Un post tiene un título, un cuerpo, un flag de publicado y un autor (el `User` con sesión). En
lugar de escribir el modelo, el controlador, las vistas y la ruta a mano, **haz el scaffold del
recurso entero** en un comando:

```bash
cargo doido generate scaffold Post \
  title:string:not_null \
  body:text:not_null \
  published:boolean:not_null \
  user:references
cargo doido db migrate
```

Ese único comando escribió:

- `app/models/post.rs` — una entidad [sea-orm](@/docs/reference/models.es.md) (con una clave
  foránea `user_id` de tipo `i64`, de `user:references`),
- una migración para la tabla `posts`,
- `app/controllers/posts_controller.rs` — un controlador CRUD completo,
- `app/views/posts/{index,show,new,edit,_form}.html.tera`,
- e **inyectó la ruta** `resources!(posts, PostsController);` en `config/routes.rs`.

Como el generador inyecta la ruta *junto* con el controlador, la ruta nunca apunta a un
controlador que aún no existe. Ahora conviértelo en un blog personalizando lo que produjo el
scaffold.

### Personalizar el modelo

Abre `app/models/post.rs` y añade una pequeña validación para rechazar posts en blanco. El trait
[`Validate`](@/docs/reference/models.es.md) de Doido acumula los errores — el resto del archivo es
exactamente lo que generó el scaffold:

```rust
// app/models/post.rs
#![allow(dead_code)]

use doido::model::sea_orm;
use doido::model::sea_orm::entity::prelude::*;
use doido::model::validation::{Errors, Validate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
    pub user_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Validate for Model {
    fn validate(&self) -> Errors {
        let mut e = Errors::new();
        e.presence("title", &self.title);
        e.length("body", &self.body, Some(10), None); // al menos 10 caracteres
        e
    }
}
```

Mantenemos `Relation` vacío y consultamos los comentarios con un filtro explícito más abajo, lo
que evita el desajuste entre la clave primaria `i32` y la foránea `i64` que un `has_many` ingenuo
sufriría.

### Personalizar el controlador

El controlador del scaffold expone las siete actions REST. Reescribe
`app/controllers/posts_controller.rs` para que la lectura sea pública, la autoría esté protegida
por login y cada nuevo post pertenezca al autor con sesión:

```rust
// app/controllers/posts_controller.rs
use crate::helpers::PostsHelper;
use crate::models::{comment, post};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use doido::model::serialization::as_json;
use serde::Deserialize;
use serde_json::json;

/// Strong params para crear/actualizar un post. El autor (`user_id`) viene de
/// la sesión, nunca del formulario.
#[derive(Deserialize)]
pub struct PostForm {
    pub title: String,
    pub body: String,
    pub published: Option<String>,
}

pub struct PostsController;

/// Interrumpe y redirige al sign-in salvo que haya alguien con sesión. El
/// sign-in guarda el id del usuario en la sesión bajo "user_id" (ve doido-auth).
async fn require_login(ctx: &mut Context) -> Result<(), Response> {
    if ctx.session().get::<i64>("user_id").is_none() {
        return Err(ctx.redirect_to("/users/sign_in"));
    }
    Ok(())
}

#[controller]
impl PostsController {
    /// GET /posts — público: solo los posts publicados.
    pub async fn index(ctx: Context) -> doido::Result<Response> {
        let posts = post::Entity::find()
            .filter(post::Column::Published.eq(true))
            .all(ctx.db())
            .await?;
        Ok(ctx.render(
            "posts/index",
            json!({
                "posts": as_json(&posts),
                "summary": PostsHelper::index_count(posts.len()),
            }),
        ))
    }

    /// GET /posts/{id} — público: el post y sus comentarios.
    pub async fn show(ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        let Some(post) = post::Entity::find_by_id(id).one(ctx.db()).await? else {
            return Ok(ctx.status(404));
        };
        let comments = comment::Entity::find()
            .filter(comment::Column::PostId.eq(i64::from(post.id)))
            .all(ctx.db())
            .await?;
        Ok(ctx.render(
            "posts/show",
            json!({ "post": as_json(&post), "comments": as_json(&comments) }),
        ))
    }

    /// GET /posts/new — la autoría está protegida por login.
    #[before_action(require_login)]
    pub async fn new(ctx: Context) -> Response {
        ctx.render("posts/new", json!({}))
    }

    /// POST /posts — crea un post que pertenece al autor con sesión.
    #[before_action(require_login)]
    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let author_id = ctx.session().get::<i64>("user_id").unwrap();
        let form: PostForm = ctx.form().await?;
        let record = post::ActiveModel {
            title: Set(form.title),
            body: Set(form.body),
            published: Set(form.published.is_some()),
            user_id: Set(author_id),
            ..Default::default()
        };
        record.insert(ctx.db()).await?;
        Ok(ctx.redirect_to("/posts"))
    }

    /// GET /posts/{id}/edit — protegido por login.
    #[before_action(require_login)]
    pub async fn edit(ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        let post = post::Entity::find_by_id(id).one(ctx.db()).await?;
        Ok(ctx.render("posts/edit", json!({ "post": as_json(&post) })))
    }

    /// PATCH/PUT /posts/{id} — protegido por login.
    #[before_action(require_login)]
    pub async fn update(mut ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        let form: PostForm = ctx.form().await?;
        if let Some(existing) = post::Entity::find_by_id(id).one(ctx.db()).await? {
            let mut record: post::ActiveModel = existing.into();
            record.title = Set(form.title);
            record.body = Set(form.body);
            record.published = Set(form.published.is_some());
            record.update(ctx.db()).await?;
        }
        Ok(ctx.redirect_to("/posts"))
    }

    /// DELETE /posts/{id} — protegido por login.
    #[before_action(require_login)]
    pub async fn destroy(ctx: Context) -> doido::Result<Response> {
        let id = parse_id(&ctx);
        post::Entity::delete_by_id(id).exec(ctx.db()).await?;
        Ok(ctx.redirect_to("/posts"))
    }
}

fn parse_id(ctx: &Context) -> i32 {
    ctx.param("id").and_then(|v| v.parse().ok()).unwrap_or_default()
}
```

El `#[before_action(require_login)]` ejecuta el guard antes de la action; devolver
`Err(response)` interrumpe la petición. `PostsHelper` es el helper que el scaffold generó junto al
controlador.

### Personalizar las vistas

Las vistas del scaffold [extienden](@/docs/reference/views.es.md) el layout generado
`app/views/layouts/application.html.tera`, que renderiza el contenido con
`{% block content %}{% endblock %}`. El JSON que pasas a `ctx.render` se vuelve el contexto de la
plantilla. Reemplaza las plantillas de index y show con un marcado con forma de blog (deja `new`,
`edit` y `_form` como los escribió el scaffold):

```html
{# app/views/posts/index.html.tera #}
{% extends "layouts/application.html.tera" %}
{% block content %}
<h1>Blog</h1>
<p>{{ summary }}</p>
{% for post in posts %}
  <article>
    <h2><a href="/posts/{{ post.id }}">{{ post.title }}</a></h2>
  </article>
{% endfor %}
{% endblock %}
```

```html
{# app/views/posts/show.html.tera #}
{% extends "layouts/application.html.tera" %}
{% block content %}
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
    <input type="text" name="author_name" required>
    <textarea name="body" required></textarea>
    <button type="submit">Comentar</button>
  </form>
</section>
{% endblock %}
```

## El modelo Comment

Un comentario pertenece a un post y lleva el nombre y el mensaje del lector. No se requiere login
para comentar, así que solo guardamos un nombre libre. Un comentario no tiene pantallas de CRUD
propias, así que un generador `model` simple basta:

```bash
cargo doido generate model Comment \
  post:references \
  author_name:string:not_null \
  body:text:not_null
cargo doido db migrate
```

El `app/models/comment.rs` generado (un `post_id` `i64`, `author_name`, `body` y un `Relation`
vacío) no necesita cambios — el `show` de arriba ya carga los comentarios de un post con un filtro
explícito por `comment::Column::PostId`.

## El controlador de comentarios

Los comentarios necesitan una sola action — create — así que usa el **generador de controlador**:

```bash
cargo doido generate controller Comments
```

Esto escribió `app/controllers/comments_controller.rs` (un stub `index` conectado a
`CommentsHelper`) e inyectó `get!("/comments", CommentsController::index);` en las rutas. Añade
una action `create` que lee el formulario e inserta un comentario para el post de la URL:

```rust
// app/controllers/comments_controller.rs
use crate::helpers::CommentsHelper;
use crate::models::comment;
use doido::controller::{controller, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct CommentForm {
    pub author_name: String,
    pub body: String,
}

pub struct CommentsController;

#[controller]
impl CommentsController {
    /// GET /comments — el stub del generador, conservado para que `CommentsHelper` siga conectado.
    pub async fn index(ctx: doido::controller::Context) -> Response {
        ctx.json(json!({ "comments": CommentsHelper::index_count(0) }))
    }

    /// POST /posts/{post_id}/comments — no requiere login para comentar.
    pub async fn create(mut ctx: doido::controller::Context) -> doido::Result<Response> {
        let post_id: i64 = ctx
            .param("post_id")
            .and_then(|v| v.parse().ok())
            .unwrap_or_default();
        let form: CommentForm = ctx.form().await?;
        let record = comment::ActiveModel {
            post_id: Set(post_id),
            author_name: Set(form.author_name),
            body: Set(form.body),
            ..Default::default()
        };
        record.insert(ctx.db()).await?;
        Ok(ctx.redirect_to(format!("/posts/{post_id}")))
    }
}
```

## Rutas

Como los generadores inyectaron una ruta con cada controlador, `config/routes.rs` ya conoce a
`PostsController` y `CommentsController` — ninguna ruta nombra un controlador que no exista. Dos
ediciones terminan el cableado: apunta la portada al blog (moviendo el endpoint de demostración de
Doido a un lado) y añade la ruta anidada de comentario junto al stub que dejó el generador de
controlador:

```rust
// config/routes.rs
use crate::controllers::CommentsController;
use crate::controllers::HelloController;
use crate::controllers::PostsController;
use doido::controller::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        root!(PostsController::index);                              // GET / — el blog
        get!("/hello", HelloController::index);                    // demo de Doido, movida a un lado

        resources!(posts, PostsController);                        // CRUD de /posts (del scaffold)
        get!("/comments", CommentsController::index);              // stub de `generate controller`
        post!("/posts/{post_id}/comments", CommentsController::create);

        // las rutas sign-in / sign-up / sign-out bajo /users las inyectó --auth — déjalas.
    }
}
```

Fíjate en el parámetro de ruta al estilo axum `{post_id}` — léelo en la action con
`ctx.param("post_id")`. Ve [Controladores y enrutamiento](@/docs/reference/controllers.es.md) para
la DSL completa.

## Ejecutar

```bash
cargo doido server
```

Ahora recorre el flujo completo:

1. Visita `/users/sign_up` y registra la cuenta del autor.
2. Ve a `/posts/new`, escribe un post y publícalo.
3. Abre `/` — el post publicado aparece. Haz clic hasta `/posts/{id}`.
4. Deja un comentario; aparece bajo el post.

Cerrar sesión (`DELETE /users/sign_out`) y volver a `/posts/new` te devuelve a la página de
sign-in — eso es `require_login` haciendo su trabajo.

## Pruebas

Los generadores dejan stubs de prueba (`tests/post_model_test.rs`, `tests/posts_controller_test.rs`);
ejecútalos con `cargo test`. Las validaciones son lógica pura, así que no necesitan base de datos:

```rust
// tests/post_validation_test.rs
#[path = "../app/models/mod.rs"]
mod models;

use doido::model::validation::Validate;
use models::post::Model;

#[test]
fn rejects_a_blank_post() {
    let post = Model {
        id: 0,
        title: String::new(),      // faltante
        body: "too short".into(),  // < 10 caracteres
        published: false,
        user_id: 1,
    };

    assert!(!post.is_valid());
    // post.validate().full_messages() lista los errores legibles.
}
```

Para pruebas de petición y de auth (montar el router real, dirigir una sesión con login), ve la
[referencia de Pruebas](@/docs/reference/auth.es.md#pruebas). El flujo completo de arriba — los
comandos de generador exactos más estas personalizaciones — también corre como un e2e de release
(`blog_tutorial`), así que este tutorial no se pudre en silencio.

## Próximos pasos

- **[Generadores y CLI](@/docs/reference/generators.es.md)** — cada generador, incluyendo
  `scaffold` y `resource`, y el estándar de tutoriales que estos pasos siguen.
- **[Modelos](@/docs/reference/models.es.md)** — asociaciones, migraciones, validaciones y factories.
- **[Controladores y enrutamiento](@/docs/reference/controllers.es.md)** — filtros, strong parameters y la DSL `routes!`.
- **[Vistas](@/docs/reference/views.es.md)** — layouts, partials y helpers de vista.
- **[Auth](@/docs/reference/auth.es.md)** — sesiones, JWT, OAuth, 2FA y los extractors.
