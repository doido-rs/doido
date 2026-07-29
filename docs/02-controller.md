# doido-controller — Spec

Rails analogue: **Action Controller**

> **Status (2026-07-28): mostly done.** `#[controller]`, filters (`before/after/around_action`,
> `skip_before_action`, `only/except`), typed params + strong params (`permit`/`require`),
> `respond_to`/format negotiation, `render`/`redirect_to`/`json`/`status`, and cookies are
> implemented (see also spec 07 for the middleware stack). **Open:** `ctx.session` is **not**
> exposed on `Context` (sessions live only in the middleware `SessionStore`), and flash
> messages are not surfaced. `.layout()`/`.no_layout()` live on `ViewResponse` (spec 04),
> not on `Context`. See [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved in interview)

- **Controller abstraction:** `#[controller]` derive macro generates the `Controller` trait impl boilerplate; actions are plain `async fn` on the struct
- **Filters:** both attribute macros on action methods **and** Tower middleware layers at router level — two complementary mechanisms

## Macro Design

```rust
#[controller]
struct PostsController;

impl PostsController {
    // attribute macro filter — runs before this action only
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

## Two Filter Mechanisms

### 1. Attribute macros (action-level, inside controller)

- `#[before_action(fn_name)]` — runs before the action
- `#[before_action(fn_name, only = [action1, action2])]` — scoped to actions
- `#[after_action(fn_name)]` — runs after the action
- Filter fn signature: `async fn name(ctx: &mut Context) -> Result<(), Response>`
- Returning `Err(response)` halts the chain and returns early (like Rails `render` in a filter)

### 2. Tower middleware layers (router-level)

- Applied via the `routes!` DSL or axum `.layer()`
- Affects all actions in a controller or entire namespace
- Examples: rate limiting, auth, request ID, CORS
- Executes **before** attribute-macro filters in the stack

## `#[controller]` Macro Responsibilities

- Implements `Controller` trait on the struct
- Wires action methods to the route handler signature axum expects
- Collects `#[before_action]` / `#[after_action]` attributes and generates filter chain per action
- Generates typed `Context` injection for each action

## `Context` — Request Context Object

```rust
// What ctx provides inside an action
ctx.params::<T>()          // typed param deserialization (path + query + body)
ctx.db                     // database connection handle
ctx.session                // session store access
ctx.render(template, data) // delegates to doido-view
ctx.redirect_to(path)      // 302 redirect helper
ctx.json(data)             // JSON response helper
ctx.status(code)           // set response status
```

## Open Questions (remaining)

- [ ] Strong params (explicit whitelist like Rails `permit`)? Or rely on serde `#[serde(deny_unknown_fields)]`?
- [ ] Flash messages — session-backed, how surfaced in views?
- [ ] CSRF protection — middleware layer or controller concern?

## Known Requirements

- Each controller is a struct annotated with `#[controller]`
- Actions are `async fn(ctx: Context) -> Response`
- Params strongly typed via serde deserialization inside `Context`
- Response helpers on `Context`: `render`, `redirect_to`, `json`, `status`
- `#[before_action]` / `#[after_action]` attribute macros on action methods
- Tower middleware at router level for cross-cutting concerns
- Test helper: construct `Context` directly without HTTP layer

## TDD Surface

- Unit test: call action directly with a fabricated `Context`, assert response
- Test `#[before_action]` halts chain and returns early when filter returns `Err`
- Test `#[before_action(fn, only = [...])` applies only to specified actions
- Test `#[after_action]` fires after action completes
- Test `ctx.params::<T>()` succeeds with valid input, errors with invalid
- Test `ctx.render(...)` delegates to `doido-view` with correct template + assigns
- Test `ctx.redirect_to(...)` returns 302 with correct `Location` header
- Integration test: router + controller + filters, full HTTP request via test client
- Integration test: middleware layer at router level runs before attribute filters
