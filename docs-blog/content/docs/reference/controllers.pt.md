+++
title = "Controllers & roteamento"
description = "A DSL routes!, recursos RESTful, controllers, filtros e o Context da requisição."
weight = 4
+++

> **Especificação de design:** [`docs/01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md)
> e [`docs/02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md).
> Este guia documenta a API como implementada em `doido-controller`.

**Análogo no Rails: Action Dispatch + Action Controller.** Um **router** mapeia URLs para
**actions de controller**; cada action é uma `async fn` que recebe um `Context` tipado da
requisição e retorna um `Response`. Por baixo é construído sobre `axum::Router`, mas você
trabalha pela macro `routes!` e pela macro `#[controller]`, e não pelo axum cru.

## Visão geral

```rust
use doido::controller::{controller, routes, Context, Response};
```

## A DSL `routes!`

Declare as rotas em `config/routes.rs`. A DSL suporta rotas por verbo HTTP, recursos
RESTful, recursos singulares, agrupamento por path/módulo, uma rota raiz, redirects e a
montagem de sub-routers.

```rust
use doido::controller::{routes, axum};

pub fn router() -> axum::Router {
    routes! {
        root!(PagesController::home);              // GET /
        get!("/about", PagesController::about);
        post!("/login", SessionsController::create);

        resources!(posts, PostsController);        // todas as 7 rotas REST
        resources!(comments, CommentsController, only: [index, show]);
        resources!(admin, AdminController, except: [destroy]);
        resources!(posts, PostsController, member: [publish], collection: [search]);

        resource!(profile, ProfileController);     // singular (sem :id)

        namespace!(api, {                          // prefixo de path E de módulo
            resources!(users, Api::UsersController);
        });
        scope!("/v2", {                            // apenas prefixo de path
            resources!(articles, V2::ArticlesController);
        });

        redirect!("/old", "/new");                 // redirect permanente
        mount!("/metrics", metrics_router());      // monta um sub-router
    }
}
```

Macros de verbo suportadas: `get!`, `post!`, `put!`, `patch!`, `delete!`.

### As 7 rotas REST

`resources!(posts, PostsController)` gera todas as sete rotas RESTful, cada uma com um
helper de URL em tempo de compilação:

| Helper | Método | Path | Action |
|--------|--------|------|--------|
| `posts_path()` | GET | `/posts` | index |
| `new_post_path()` | GET | `/posts/new` | new |
| `post_path(id)` | GET | `/posts/:id` | show |
| `post_path(id)` | POST | `/posts` | create |
| `edit_post_path(id)` | GET | `/posts/:id/edit` | edit |
| `post_path(id)` | PATCH | `/posts/:id` | update |
| `post_path(id)` | DELETE | `/posts/:id` | destroy |

`only:` / `except:` restringem o conjunto gerado; `member:` / `collection:` adicionam
rotas extras (rotas de member recebem um `:id`, rotas de collection não).

## Controllers

Um controller é uma struct anotada com `#[controller]`; as actions são `async fn(ctx:
Context) -> Response`. A rota despacha para a action cujo nome de método corresponde
(convenção sobre configuração).

```rust
use doido::controller::{controller, Context, Response};
use serde_json::json;

pub struct PostsController;

#[controller]
impl PostsController {
    async fn index(ctx: Context) -> Response {
        let posts = post::Entity::find().all(ctx.db()).await.unwrap_or_default();
        ctx.render("posts/index", json!({ "posts": doido::model::serialization::as_json(&posts) }))
    }

    async fn show(ctx: Context) -> Response {
        match ctx.param("id") {
            Some(id) => ctx.json(json!({ "id": id })),
            None => ctx.status(404),
        }
    }
}
```

## O `Context` da requisição

Tudo o que uma action precisa está no `ctx`.

**Lendo a entrada:**

```rust
ctx.param("id");                         // Option<&str> — um segmento do path
ctx.params::<Filters>()?;                // query string tipada (GET)
ctx.query_params();                      // Params sem tipo (para require/permit)
ctx.form::<CreatePost>().await?;         // corpo de formulário URL-encoded
ctx.body_json::<CreatePost>().await?;    // corpo JSON
ctx.header("authorization");             // Option<&HeaderValue>
ctx.db();                                // &'static DatabaseConnection
```

**Produzindo uma resposta:**

```rust
ctx.render("posts/show", json!({ "post": post }));   // 200 HTML via doido-view
ctx.json(json!({ "ok": true }));                     // 200 JSON
ctx.redirect_to(post_path(post.id));                 // 302
ctx.status(422);                                     // status com corpo vazio
ctx.send_data(bytes, "application/pdf", Some("report.pdf")); // download
ctx.send_file("storage/report.pdf", None).await;     // envia um arquivo em streaming
```

## Filtros

Filtros por macro de atributo rodam em volta das actions. Um `before_action`/`after_action`
tem a assinatura `async fn(ctx: &mut Context) -> Result<(), Response>`; retornar
`Err(response)` interrompe a cadeia e retorna cedo (o padrão `render`-e-retorna do Rails).
Restrinja com `only = [...]` / `except = [...]`, e cancele um filtro herdado com
`#[skip_before_action(...)]`.

```rust
async fn require_auth(ctx: &mut Context) -> Result<(), Response> {
    if ctx.header("x-auth-token").is_none() {
        return Err(ctx.status(401)); // interrompe
    }
    Ok(())
}

#[controller]
impl PostsController {
    #[before_action(require_auth, except = [index, show])]
    async fn create(ctx: Context) -> Response { ctx.status(201) }

    #[before_action(require_auth)]
    #[skip_before_action(require_auth)] // exclui esta action do filtro
    async fn index(ctx: Context) -> Response { ctx.status(200) }
}
```

Um `around_action` envolve a action e é dono da resposta:

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

## Parâmetros fortes

`query_params()` retorna um `Params` do qual você pode `require` uma chave e `permit` uma
lista de campos permitidos — a proteção contra mass-assignment do Rails.

```rust
let post_params = ctx.query_params()
    .require("post")?          // precisa estar presente
    .permit(&["title", "body"]); // descarta o resto

let create: CreatePost = post_params.deserialize()?;
```

## Negociação de conteúdo

`respond_to()` escolhe um ramo com base no header `Accept` da requisição;
`negotiated_format()` retorna o `Format` resolvido (`Html`, `Json` ou `Any`).

```rust
async fn show(ctx: Context) -> Response {
    ctx.respond_to()
        .html(|| ctx.render("posts/show", json!({ "post": post })))
        .json(|| ctx.json(json!({ "post": post })))
        .finish()
}
```

## GET condicional (ETags)

`fresh_when()` retorna um `304 Not Modified` cedo quando os validadores do cliente ainda
batem; `etag_matches()` verifica um valor `If-None-Match`.

```rust
async fn show(ctx: Context) -> Response {
    if let Some(not_modified) = ctx.fresh_when(Some(&post.etag), None) {
        return not_modified; // 304
    }
    ctx.json(json!({ "post": post }))
}
```

## Testes

As actions são funções async comuns, então você pode montá-las em um `axum::Router` e
dirigi-las com um cliente de teste, ou construir um `Context` diretamente. Filtros,
parâmetros e respostas são todos verificáveis sem um servidor de verdade.

## Veja também

- [Middleware & sessões](@/docs/reference/middleware.pt.md) — a stack Tower, sessões, flash, CSRF, CORS.
- [Views](@/docs/reference/views.pt.md) — para onde `ctx.render(...)` delega.
- [Models](@/docs/reference/models.pt.md) — usando `ctx.db()` dentro das actions.
