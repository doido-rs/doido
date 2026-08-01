+++
title = "Controllers & routing"
description = "The routes! DSL, RESTful resources, controllers, filters, and the request Context."
weight = 4
aliases = ['/docs/guides/controllers/']

+++

> **Design spec:** [`docs/01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md)
> and [`docs/02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md).
> This guide documents the API as implemented in `doido-controller`.

**Rails analogue: Action Dispatch + Action Controller.** A **router** maps URLs to
**controller actions**; each action is an `async fn` that takes a typed request `Context`
and returns a `Response`. It is built on `axum::Router` under the hood, but you work
through the `routes!` macro DSL and the `#[controller]` macro rather than raw axum.

## At a glance

```rust
use doido_controller::{controller, routes, Context, Response};
```

## The `routes!` DSL

Declare routes in `config/routes.rs`. The DSL supports HTTP-verb routes, RESTful
resources, singular resources, path/module grouping, a root route, redirects, and mounting
sub-routers.

```rust
use doido_controller::routes;

pub fn router() -> doido_controller::axum::Router {
    routes! {
        root!(PagesController::home);              // GET /
        get!("/about", PagesController::about);
        post!("/login", SessionsController::create);

        resources!(posts, PostsController);        // all 7 REST routes
        resources!(comments, CommentsController, only: [index, show]);
        resources!(admin, AdminController, except: [destroy]);
        resources!(posts, PostsController, member: [publish], collection: [search]);

        resource!(profile, ProfileController);     // singular (no :id)

        namespace!(api, {                          // path AND module prefix
            resources!(users, Api::UsersController);
        });
        scope!("/v2", {                            // path prefix only
            resources!(articles, V2::ArticlesController);
        });

        redirect!("/old", "/new");                 // permanent redirect
        mount!("/metrics", metrics_router());      // mount a sub-router
    }
}
```

Supported verb macros: `get!`, `post!`, `put!`, `patch!`, `delete!`.

### The 7 REST routes

`resources!(posts, PostsController)` generates all seven RESTful routes, each with a
compile-time URL helper:

| Helper | Method | Path | Action |
|--------|--------|------|--------|
| `posts_path()` | GET | `/posts` | index |
| `new_post_path()` | GET | `/posts/new` | new |
| `post_path(id)` | GET | `/posts/:id` | show |
| `post_path(id)` | POST | `/posts` | create |
| `edit_post_path(id)` | GET | `/posts/:id/edit` | edit |
| `post_path(id)` | PATCH | `/posts/:id` | update |
| `post_path(id)` | DELETE | `/posts/:id` | destroy |

`only:` / `except:` restrict the generated set; `member:` / `collection:` add extra
routes (member routes take an `:id`, collection routes don't).

## Controllers

A controller is a struct annotated with `#[controller]`; actions are `async fn(ctx:
Context) -> Response`. The route dispatches to the action whose method name matches
(convention over configuration).

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

## The request `Context`

Everything an action needs is on `ctx`.

**Reading input:**

```rust
ctx.param("id");                         // Option<&str> — a path segment
ctx.params::<Filters>()?;                // typed query string (GET)
ctx.query_params();                      // untyped query Params (for require/permit)
ctx.form::<CreatePost>().await?;         // URL-encoded form body
ctx.body_json::<CreatePost>().await?;    // JSON body
ctx.header("authorization");             // Option<&HeaderValue>
ctx.db();                                // &'static DatabaseConnection
```

**Producing a response:**

```rust
ctx.render("posts/show", json!({ "post": post }));   // 200 HTML via doido-view
ctx.json(json!({ "ok": true }));                     // 200 JSON
ctx.redirect_to(post_path(post.id));                 // 302
ctx.status(422);                                     // status with empty body
ctx.send_data(bytes, "application/pdf", Some("report.pdf")); // download
ctx.send_file("storage/report.pdf", None).await;     // stream a file
```

## Filters

Attribute-macro filters run around actions. A `before_action`/`after_action` has the
signature `async fn(ctx: &mut Context) -> Result<(), Response>`; returning `Err(response)`
halts the chain and returns early (the Rails `render`-and-return pattern). Scope with
`only = [...]` / `except = [...]`, and cancel an inherited filter with
`#[skip_before_action(...)]`.

```rust
async fn require_auth(ctx: &mut Context) -> Result<(), Response> {
    if ctx.header("x-auth-token").is_none() {
        return Err(ctx.status(401)); // halt
    }
    Ok(())
}

#[controller]
impl PostsController {
    #[before_action(require_auth, except = [index, show])]
    async fn create(ctx: Context) -> Response { ctx.status(201) }

    #[before_action(require_auth)]
    #[skip_before_action(require_auth)] // opt this action back out
    async fn index(ctx: Context) -> Response { ctx.status(200) }
}
```

An `around_action` brackets the action and owns the response:

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

## Strong parameters

`query_params()` returns a `Params` you can `require` a key from and `permit` an allowlist
of fields — the Rails mass-assignment guard.

```rust
let post_params = ctx.query_params()
    .require("post")?          // must be present
    .permit(&["title", "body"]); // drop everything else

let create: CreatePost = post_params.deserialize()?;
```

## Content negotiation

`respond_to()` picks a branch based on the request's `Accept` header; `negotiated_format()`
returns the resolved `Format` (`Html`, `Json`, or `Any`).

```rust
async fn show(ctx: Context) -> Response {
    ctx.respond_to()
        .html(|| ctx.render("posts/show", json!({ "post": post })))
        .json(|| ctx.json(json!({ "post": post })))
        .finish()
}
```

## Conditional GET (ETags)

`fresh_when()` returns a `304 Not Modified` early when the client's validators still match;
`etag_matches()` checks an `If-None-Match` value.

```rust
async fn show(ctx: Context) -> Response {
    if let Some(not_modified) = ctx.fresh_when(Some(&post.etag), None) {
        return not_modified; // 304
    }
    ctx.json(json!({ "post": post }))
}
```

## Testing

Actions are plain async functions, so you can mount them on an `axum::Router` and drive
them with a test client, or build a `Context` directly. Filters, params, and responses are
all assertable without a live server.

## See also

- [Middleware & sessions](@/docs/reference/middleware.md) — the Tower stack, sessions, flash, CSRF, CORS.
- [Views](@/docs/reference/views.md) — what `ctx.render(...)` delegates to.
- [Models](@/docs/reference/models.md) — using `ctx.db()` inside actions.
