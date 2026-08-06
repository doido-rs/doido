+++
title = "Controller helpers"
description = "Auxiliary modules in app/helpers/ — the #[helper] macro, shared logic for controllers, and the helper generator."
weight = 5
aliases = ['/docs/guides/helpers/']

+++

> **Implementation:** `doido-controller` (`Helper` trait + `#[helper]` macro).
> This guide documents the API as implemented today.

**Rails analogue: controller helpers** (`app/helpers/`). Helpers are plain Rust
modules that hold **shared logic your controllers import** — formatting, small
transformations, authorization checks that don't belong in a single action, and
other utilities you want to reuse across controllers.

They are **not** the same as [view helpers](@/docs/reference/views.md) in
`doido-view` (`link_to`, `form_tag`, …), which build HTML for templates. Controller
helpers live under `app/helpers/` and are imported explicitly in controller code.

## At a glance

```rust
use doido::controller::helper;
```

## Layout

Every generated app includes a central helpers directory:

```
app/
├── controllers/
├── helpers/
│   ├── mod.rs                  ← registry (@generated-helpers marker)
│   └── application_helper.rs   ← default app-wide helper
└── models/
```

`src/main.rs` wires the module with `#[path = "../app/helpers/mod.rs"] mod helpers;`
so controllers import helpers as `crate::helpers::…`.

## Defining a helper

Mark a struct with `#[helper]`. The macro implements the `Helper` trait and adds
`helper_name()` — the snake_case name derived from the struct (`PostsHelper` →
`"posts_helper"`), matching the file convention `app/helpers/posts_helper.rs`.

```rust
use doido::controller::helper;

#[helper]
pub struct PostsHelper;

impl PostsHelper {
    pub fn format_title(title: &str) -> String {
        title.trim().to_uppercase()
    }

    pub fn excerpt(body: &str, max_len: usize) -> String {
        if body.len() <= max_len {
            body.to_string()
        } else {
            format!("{}…", &body[..max_len])
        }
    }
}
```

## Using a helper in a controller

Import the helper at the top of your controller file and call its associated
functions from any action:

```rust
use crate::helpers::PostsHelper;
use doido::controller::{controller, Context, Response};
use serde_json::json;

pub struct PostsController;

#[controller]
impl PostsController {
    async fn index(ctx: Context) -> Response {
        let title = PostsHelper::format_title("hello");
        ctx.json(json!({ "title": title }))
    }
}
```

The generated `HelloController` uses the same pattern with `ApplicationHelper`:

```rust
use crate::helpers::ApplicationHelper;

#[controller]
impl HelloController {
    pub async fn index(ctx: Context) -> Response {
        ctx.json(json!({
            "message": ApplicationHelper::greet("world")
        }))
    }
}
```

`GET /` then returns:

```json
{ "message": "Hello, world!" }
```

## Generating a helper

```bash
cargo doido generate helper Posts
```

This writes:

| Path | Purpose |
|------|---------|
| `app/helpers/posts_helper.rs` | `PostsHelper` struct stub with `#[helper]` |
| `app/helpers/mod.rs` | Registers `pub mod posts_helper;` and `pub use posts_helper::PostsHelper;` |
| `tests/posts_helper_test.rs` | Smoke test for `helper_name()` |

The generator accepts `Posts` or `PostsHelper` as the name — both produce
`PostsHelper` in `posts_helper.rs` (no double `_helper` suffix).

```bash
cargo doido generate helper Posts        # → PostsHelper
cargo doido generate helper PostsHelper  # → PostsHelper (unchanged)
```

## When to use a helper

| Use a controller helper when… | Prefer something else when… |
|------------------------------|----------------------------|
| Logic is reused across several controllers | Logic belongs to one model → put it on the model |
| You need a pure function with no HTTP context | You need request/session data → use a filter or inline in the action |
| You want a named, testable unit separate from actions | You are building HTML for templates → use [view helpers](@/docs/reference/views.md) |

## Testing

Helpers are plain Rust modules — unit-test them directly without HTTP:

```rust
use crate::helpers::PostsHelper;

#[test]
fn format_title_uppercases_and_trims() {
    assert_eq!(PostsHelper::format_title("  hi  "), "HI");
}

#[test]
fn helper_name_matches_file_convention() {
    assert_eq!(PostsHelper::helper_name(), "posts_helper");
}
```

Integration tests can also mount a controller that calls the helper and assert the
HTTP response, the same way you test any other action.

## See also

- [Controllers & routing](@/docs/reference/controllers.md) — actions, filters, and `Context`.
- [Views](@/docs/reference/views.md) — HTML helpers for templates (distinct from controller helpers).
- [Generators & CLI](@/docs/reference/generators.md) — `cargo doido generate helper`.
