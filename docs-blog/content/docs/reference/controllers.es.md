+++
title = "Controladores y enrutamiento"
description = "La DSL routes!, recursos RESTful, controladores, filtros y el Context de la petición."
weight = 4
+++

> **Especificación de diseño:** [`docs/01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md)
> y [`docs/02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md).
> Esta guía documenta la API tal como está implementada en `doido-controller`.

**Análogo en Rails: Action Dispatch + Action Controller.** Un **router** mapea URLs a
**actions de controlador**; cada action es una `async fn` que recibe un `Context` tipado de
la petición y devuelve un `Response`. Por debajo se construye sobre `axum::Router`, pero
trabajas mediante la macro `routes!` y la macro `#[controller]`, no con axum en crudo.

## Vistazo general

```rust
use doido_controller::{controller, routes, Context, Response};
```

## La DSL `routes!`

Declara las rutas en `config/routes.rs`. La DSL soporta rutas por verbo HTTP, recursos
RESTful, recursos singulares, agrupación por path/módulo, una ruta raíz, redirecciones y el
montaje de sub-routers.

```rust
use doido_controller::routes;

pub fn router() -> doido_controller::axum::Router {
    routes! {
        root!(PagesController::home);              // GET /
        get!("/about", PagesController::about);
        post!("/login", SessionsController::create);

        resources!(posts, PostsController);        // las 7 rutas REST
        resources!(comments, CommentsController, only: [index, show]);
        resources!(admin, AdminController, except: [destroy]);
        resources!(posts, PostsController, member: [publish], collection: [search]);

        resource!(profile, ProfileController);     // singular (sin :id)

        namespace!(api, {                          // prefijo de path Y de módulo
            resources!(users, Api::UsersController);
        });
        scope!("/v2", {                            // solo prefijo de path
            resources!(articles, V2::ArticlesController);
        });

        redirect!("/old", "/new");                 // redirección permanente
        mount!("/metrics", metrics_router());      // monta un sub-router
    }
}
```

Macros de verbo soportadas: `get!`, `post!`, `put!`, `patch!`, `delete!`.

### Las 7 rutas REST

`resources!(posts, PostsController)` genera las siete rutas RESTful, cada una con un helper
de URL en tiempo de compilación:

| Helper | Método | Path | Action |
|--------|--------|------|--------|
| `posts_path()` | GET | `/posts` | index |
| `new_post_path()` | GET | `/posts/new` | new |
| `post_path(id)` | GET | `/posts/:id` | show |
| `post_path(id)` | POST | `/posts` | create |
| `edit_post_path(id)` | GET | `/posts/:id/edit` | edit |
| `post_path(id)` | PATCH | `/posts/:id` | update |
| `post_path(id)` | DELETE | `/posts/:id` | destroy |

`only:` / `except:` restringen el conjunto generado; `member:` / `collection:` añaden rutas
extra (las rutas de member reciben un `:id`, las de collection no).

## Controladores

Un controlador es una struct anotada con `#[controller]`; las actions son `async fn(ctx:
Context) -> Response`. La ruta despacha a la action cuyo nombre de método coincide
(convención sobre configuración).

```rust
use doido_controller::{controller, Context, Response};
use serde_json::json;

pub struct PostsController;

#[controller]
impl PostsController {
    async fn index(ctx: Context) -> Response {
        let posts = post::Entity::find().all(ctx.db()).await.unwrap_or_default();
        ctx.render("posts/index", json!({ "posts": doido_model::serialization::as_json(&posts) }))
    }

    async fn show(ctx: Context) -> Response {
        match ctx.param("id") {
            Some(id) => ctx.json(json!({ "id": id })),
            None => ctx.status(404),
        }
    }
}
```

## El `Context` de la petición

Todo lo que una action necesita está en `ctx`.

**Leer la entrada:**

```rust
ctx.param("id");                         // Option<&str> — un segmento del path
ctx.params::<Filters>()?;                // query string tipada (GET)
ctx.query_params();                      // Params sin tipo (para require/permit)
ctx.form::<CreatePost>().await?;         // cuerpo de formulario URL-encoded
ctx.body_json::<CreatePost>().await?;    // cuerpo JSON
ctx.header("authorization");             // Option<&HeaderValue>
ctx.db();                                // &'static DatabaseConnection
```

**Producir una respuesta:**

```rust
ctx.render("posts/show", json!({ "post": post }));   // 200 HTML vía doido-view
ctx.json(json!({ "ok": true }));                     // 200 JSON
ctx.redirect_to(post_path(post.id));                 // 302
ctx.status(422);                                     // estado con cuerpo vacío
ctx.send_data(bytes, "application/pdf", Some("report.pdf")); // descarga
ctx.send_file("storage/report.pdf", None).await;     // envía un archivo en streaming
```

## Filtros

Los filtros por macro de atributo se ejecutan alrededor de las actions. Un
`before_action`/`after_action` tiene la firma `async fn(ctx: &mut Context) -> Result<(),
Response>`; devolver `Err(response)` detiene la cadena y retorna pronto (el patrón
`render`-y-retorna de Rails). Restríngelos con `only = [...]` / `except = [...]`, y cancela
un filtro heredado con `#[skip_before_action(...)]`.

```rust
async fn require_auth(ctx: &mut Context) -> Result<(), Response> {
    if ctx.header("x-auth-token").is_none() {
        return Err(ctx.status(401)); // detiene
    }
    Ok(())
}

#[controller]
impl PostsController {
    #[before_action(require_auth, except = [index, show])]
    async fn create(ctx: Context) -> Response { ctx.status(201) }

    #[before_action(require_auth)]
    #[skip_before_action(require_auth)] // excluye esta action del filtro
    async fn index(ctx: Context) -> Response { ctx.status(200) }
}
```

Un `around_action` envuelve la action y es dueño de la respuesta:

```rust
async fn timed(ctx: &mut Context, run: impl AsyncFnOnce(&mut Context) -> Response) -> Response {
    let mut resp = run(ctx).await;
    resp.headers_mut().insert("x-served-by", "doido".parse().unwrap());
    resp
}

#[controller]
impl PostsController {
    #[around_action(timed)]
    async fn show(ctx: Context) -> Response { ctx.status(200) }
}
```

## Parámetros fuertes

`query_params()` devuelve un `Params` del que puedes `require` una clave y `permit` una
lista de campos permitidos — la protección contra mass-assignment de Rails.

```rust
let post_params = ctx.query_params()
    .require("post")?          // debe estar presente
    .permit(&["title", "body"]); // descarta el resto

let create: CreatePost = post_params.deserialize()?;
```

## Negociación de contenido

`respond_to()` elige una rama según el header `Accept` de la petición;
`negotiated_format()` devuelve el `Format` resuelto (`Html`, `Json` o `Any`).

```rust
async fn show(ctx: Context) -> Response {
    ctx.respond_to()
        .html(|| ctx.render("posts/show", json!({ "post": post })))
        .json(|| ctx.json(json!({ "post": post })))
        .finish()
}
```

## GET condicional (ETags)

`fresh_when()` devuelve un `304 Not Modified` de forma temprana cuando los validadores del
cliente aún coinciden; `etag_matches()` comprueba un valor `If-None-Match`.

```rust
async fn show(ctx: Context) -> Response {
    if let Some(not_modified) = ctx.fresh_when(Some(&post.etag), None) {
        return not_modified; // 304
    }
    ctx.json(json!({ "post": post }))
}
```

## Pruebas

Las actions son funciones async normales, así que puedes montarlas en un `axum::Router` y
dirigirlas con un cliente de pruebas, o construir un `Context` directamente. Filtros,
parámetros y respuestas son todos verificables sin un servidor real.

## Véase también

- [Middleware y sesiones](@/docs/reference/middleware.es.md) — la stack Tower, sesiones, flash, CSRF, CORS.
- [Vistas](@/docs/reference/views.es.md) — a qué delega `ctx.render(...)`.
- [Modelos](@/docs/reference/models.es.md) — usar `ctx.db()` dentro de las actions.
