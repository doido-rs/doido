# doido-controller — Spec

Rails analogue: **Action Controller**

> **Status (2026-08-06): mostly done.** `#[controller]`, filters (`before/after/around_action`,
> `skip_before_action`, `only/except`), typed params + strong params (`permit`/`require`),
> `respond_to`/format negotiation, `render`/`redirect_to`/`json`/`status`, cookies, and
> **controller helpers** (`#[helper]`, `app/helpers/`) are implemented (see also spec 07 for
> the middleware stack). **Open:** `ctx.session` is **not**
> exposed on `Context` (sessions live only in the middleware `SessionStore`), and flash
> messages are not surfaced. `.layout()`/`.no_layout()` live on `ViewResponse` (spec 04),
> not on `Context`. See [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved in interview)

- **Controller abstraction:** `#[controller]` derive macro generates the `Controller` trait impl boilerplate; actions are plain `async fn` on the struct
- **Filters:** both attribute macros on action methods **and** Tower middleware layers at router level — two complementary mechanisms
- **axum import path:** workspace crates and `routes!`/`#[controller]` generated code use `doido_controller::axum`; generated apps may use `doido::controller::axum`. Do not depend on `axum` directly outside `doido-controller`.
- **Controller helpers:** shared logic lives in `app/helpers/` as structs annotated with `#[helper]`; controllers import them explicitly (`use crate::helpers::PostsHelper`). Distinct from **view helpers** in `doido-view` (spec 04), which build HTML for templates.

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

## Controller helpers (`app/helpers/`)

Rails analogue: **Action Controller helpers** — auxiliary modules imported by controllers
(not auto-included in views). Use them for formatting, small transformations, and other
logic reused across controller actions.

**Not** the same as `doido-view` helpers (`link_to`, `form_tag`, …) — those live in
`doido_view::helpers` and serve templates (spec 04).

### Layout

Generated apps centralize helpers under `app/helpers/`:

```
app/helpers/
  mod.rs                    ← registry (@generated-helpers marker)
  application_helper.rs     ← default app-wide helper (from `doido new`)
  posts_helper.rs           ← e.g. from `doido generate helper Posts` or `scaffold`
```

`src/main.rs` mounts the tree with `#[path = "../app/helpers/mod.rs"] mod helpers;`
so controllers import via `crate::helpers::…`.

### `#[helper]` macro

Mark a struct with `#[helper]`. The macro implements the `Helper` trait and adds
`helper_name()` — the snake_case name derived from the struct (`PostsHelper` →
`"posts_helper"`), matching the file convention `app/helpers/posts_helper.rs`.

```rust
use doido::controller::helper;

#[helper]
pub struct PostsHelper;

impl PostsHelper {
    pub fn label() -> &'static str {
        "posts"
    }

    pub fn index_count(count: usize) -> String {
        format!("{count} {}", Self::label())
    }
}
```

Re-exported from `doido-controller` (generated apps: `doido::controller::helper`).

### Using a helper in a controller

Import at the top of the controller file and call associated functions from actions:

```rust
use crate::helpers::PostsHelper;

#[controller]
impl PostsController {
    async fn index(ctx: Context) -> Response {
        let posts = /* … */;
        ctx.render("posts/index", json!({
            "posts": posts,
            "summary": PostsHelper::index_count(posts.len()),
        }))
    }
}
```

`scaffold`, `resource`, and `controller` generators emit a matching helper and wire the
`index` action to call `{Plural}Helper::index_count` (see spec 06b).

### Generator

```sh
doido generate helper Posts        # → PostsHelper in app/helpers/posts_helper.rs
doido generate helper PostsHelper  # → same (no double `_helper` suffix)
```

Registers `pub mod posts_helper;` and `pub use posts_helper::PostsHelper;` in
`app/helpers/mod.rs`.

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
- Controller helpers in `app/helpers/` with `#[helper]` + `Helper` trait
- Test helper: construct `Context` directly without HTTP layer

## TDD Surface

- Unit test: call action directly with a fabricated `Context`, assert response
- Test `#[before_action]` halts chain and returns early when filter returns `Err`
- Test `#[before_action(fn, only = [...])` applies only to specified actions
- Test `#[after_action]` fires after action completes
- Test `ctx.params::<T>()` succeeds with valid input, errors with invalid
- Test `ctx.render(...)` delegates to `doido-view` with correct template + assigns
- Test `ctx.redirect_to(...)` returns 302 with correct `Location` header
- Unit test: `#[helper]` implements `Helper::helper_name()` from struct name
- Unit test: controller action calls imported helper function
- Integration test: router + controller + filters, full HTTP request via test client
- Integration test: middleware layer at router level runs before attribute filters
