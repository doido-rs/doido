+++
title = "Controllers & routing"
description = "Define routes, write controllers, and use filters and the request Context."
weight = 1
+++

> **Design spec:** [`docs/01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md)
> and [`docs/02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md).
> This guide is the usage-focused companion to those specs.

Doido's request layer maps cleanly onto Rails: a **router** dispatches URLs to
**controller actions**, and each action receives a typed request `Context` and
returns a `Response`. It is built on `axum::Router` under the hood, but you work
through the `routes!` macro DSL rather than raw axum.

## Routing

Routes are declared with the `routes!` macro in `config/routes.rs`:

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

- `namespace!` prefixes **both** the path and the controller module path.
- `scope!` prefixes **only** the path.
- Supported verbs: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`.

### The 7 REST routes

`resources!(posts, PostsController)` generates all seven RESTful routes, each
with a compile-time URL helper:

| Helper | Method | Path | Action |
|--------|--------|------|--------|
| `posts_path()` | GET | `/posts` | index |
| `new_post_path()` | GET | `/posts/new` | new |
| `post_path(id)` | GET | `/posts/:id` | show |
| `post_path(id)` | POST | `/posts` | create |
| `edit_post_path(id)` | GET | `/posts/:id/edit` | edit |
| `post_path(id)` | PATCH | `/posts/:id` | update |
| `post_path(id)` | DELETE | `/posts/:id` | destroy |

Use `only:` / `except:` to restrict which of the seven are generated.

## Controllers

A controller is a struct annotated with `#[controller]`; actions are plain
`async fn` that take a `Context` and return a `Response`. The route dispatches to
the action whose method name matches the action (convention over configuration).

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

## The request `Context`

Everything an action needs is on `ctx`:

```rust
ctx.params::<T>()          // typed params (path + query + body) via serde
ctx.db                     // database connection handle
ctx.session                // session store access
ctx.render(template, data) // render a view (delegates to doido-view)
ctx.redirect_to(path)      // 302 redirect helper
ctx.json(data)             // JSON response helper
ctx.status(code)           // set the response status
```

## Two ways to filter

Doido offers two complementary filter mechanisms:

1. **Attribute-macro filters (action-level).** `#[before_action(fn)]` and
   `#[after_action(fn)]` on the controller. Scope them with
   `only = [action1, action2]`. A `before_action` has the signature
   `async fn(ctx: &mut Context) -> Result<(), Response>`; returning `Err(response)`
   halts the chain and returns early — the equivalent of `render`-and-return in a
   Rails filter.

2. **Tower middleware layers (router-level).** Applied via the `routes!` DSL or
   axum's `.layer()`, these cover cross-cutting concerns (auth, rate limiting,
   request IDs, CORS) across a whole controller or namespace. Middleware runs
   **before** attribute-macro filters.

## Testing

The controller layer is built to be tested without an HTTP server: construct a
`Context` directly and call the action, asserting on the returned `Response`. For
full-stack coverage, mount a `routes!` block and drive it with the test client.
See the TDD surface in the specs for the exact test matrix.
