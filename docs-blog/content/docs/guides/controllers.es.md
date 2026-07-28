+++
title = "Controladores y enrutamiento"
description = "Define rutas, escribe controladores y usa filtros y el Context de la petición."
weight = 1
+++

> **Especificación de diseño:** [`docs/01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md)
> y [`docs/02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md).
> Esta guía es el complemento centrado en el uso de esas especificaciones.

La capa de peticiones de Doido se corresponde de forma limpia con Rails: un
**router** despacha URLs a **actions de controlador**, y cada action recibe un
`Context` tipado y devuelve un `Response`. Por debajo se construye sobre
`axum::Router`, pero trabajas a través de la macro `routes!` en lugar de axum en
crudo.

## Enrutamiento

Las rutas se declaran con la macro `routes!` en `config/routes.rs`:

```rust
routes! {
    resources!(posts, PostsController);
    resources!(comments, CommentsController, only: [index, show]);
    resources!(admin, AdminController, except: [destroy]);

    get!("/about", PagesController::about);
    post!("/login", SessionsController::create);

    namespace!(api, {
        resources!(users, Api::UsersController);
    });

    scope!("/v2", {
        resources!(articles, V2::ArticlesController);
    });
}
```

- `namespace!` prefija **tanto** el path **como** la ruta del módulo del controlador.
- `scope!` prefija **solo** el path.
- Verbos soportados: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`.

### Las 7 rutas REST

`resources!(posts, PostsController)` genera las siete rutas RESTful, cada una con
un helper de URL en tiempo de compilación:

| Helper | Método | Path | Action |
|--------|--------|------|--------|
| `posts_path()` | GET | `/posts` | index |
| `new_post_path()` | GET | `/posts/new` | new |
| `post_path(id)` | GET | `/posts/:id` | show |
| `post_path(id)` | POST | `/posts` | create |
| `edit_post_path(id)` | GET | `/posts/:id/edit` | edit |
| `post_path(id)` | PATCH | `/posts/:id` | update |
| `post_path(id)` | DELETE | `/posts/:id` | destroy |

Usa `only:` / `except:` para restringir cuáles de las siete se generan.

## Controladores

Un controlador es una struct anotada con `#[controller]`; las actions son `async
fn` normales que reciben un `Context` y devuelven un `Response`. La ruta despacha
a la action cuyo nombre de método coincide con la action (convención sobre
configuración).

```rust
#[controller]
struct PostsController;

impl PostsController {
    #[before_action(authenticate)]
    #[before_action(find_post, only = [show, edit, update, destroy])]
    async fn index(ctx: Context) -> Response {
        let posts = Post::all(&ctx.db).await?;
        ctx.render("posts/index", json!({ "posts": posts }))
    }

    #[before_action(authenticate)]
    #[after_action(log_response)]
    async fn create(ctx: Context) -> Response {
        let params = ctx.params::<CreatePostParams>()?;
        match Post::create(&ctx.db, params).await {
            Ok(post) => ctx.redirect_to(post_path(post.id)),
            Err(_)   => ctx.render("posts/new", status = 422),
        }
    }
}
```

## El `Context` de la petición

Todo lo que una action necesita está en `ctx`:

```rust
ctx.params::<T>()          // params tipados (path + query + body) vía serde
ctx.db                     // handle de la conexión a la base de datos
ctx.session                // acceso al store de sesión
ctx.render(template, data) // renderiza una vista (delega a doido-view)
ctx.redirect_to(path)      // helper de redirect 302
ctx.json(data)             // helper de respuesta JSON
ctx.status(code)           // define el estado de la respuesta
```

## Dos formas de filtrar

Doido ofrece dos mecanismos de filtro complementarios:

1. **Filtros por macro de atributo (nivel de action).** `#[before_action(fn)]` y
   `#[after_action(fn)]` en el controlador. Restríngelos con
   `only = [action1, action2]`. Un `before_action` tiene la firma
   `async fn(ctx: &mut Context) -> Result<(), Response>`; devolver `Err(response)`
   detiene la cadena y retorna pronto — el equivalente a `render`-y-retorna en un
   filtro de Rails.

2. **Capas de middleware Tower (nivel de router).** Aplicadas vía la DSL `routes!`
   o el `.layer()` de axum, cubren preocupaciones transversales (autenticación,
   rate limiting, request IDs, CORS) en un controlador o namespace completo. El
   middleware se ejecuta **antes** de los filtros por macro de atributo.

## Pruebas

La capa de controlador está pensada para probarse sin un servidor HTTP: construye
un `Context` directamente y llama a la action, verificando el `Response`
devuelto. Para cobertura de extremo a extremo, monta un bloque `routes!` y
condúcelo con el cliente de pruebas. Consulta la superficie de TDD en las
especificaciones para la matriz de pruebas exacta.
