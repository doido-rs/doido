# doido-view — Spec

Rails analogue: **Action View**

## Decisions (resolved in interview)

- **Default template engine: Tera** (Jinja2-like, runtime, hot-reload friendly)
- **Engine is swappable** — a `TemplateEngine` trait; Tera is the default impl
- Engine selected via `doido-config` (`view.engine = "tera"` | custom)

## Template Engine Trait

```rust
// Any engine implements this trait
pub trait TemplateEngine: Send + Sync {
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String>;
    fn reload(&self) -> Result<()>;  // hot-reload templates from disk (dev only)
}
```

Built-in impls:
- `TeraEngine` — default, wraps `tera::Tera`
- Additional engines can be registered by the user at app boot

## Template Resolution Convention

Mirrors Rails `app/views/<controller>/<action>.html.erb`:

```
views/
  posts/
    index.html.tera
    show.html.tera
    new.html.tera
    edit.html.tera
  layouts/
    application.html.tera
  shared/
    _header.html.tera    ← partials prefixed with _
```

- Template key: `"posts/index"` → resolves to `views/posts/index.html.tera`
- Layout wraps content via `{{ content_for_layout }}` (Rails `yield` equivalent)
- Partials included via Tera's `{% include "shared/_header.html.tera" %}`

## Rendering from a Controller

`Context::render(template, data) -> Response` delegates to a process-global Tera
engine installed at server boot (`doido_view::init("app/views")`), mirroring the
DB pool. The template key resolves under `app/views` with the `.html.tera`
suffix (`"posts/index"` → `app/views/posts/index.html.tera`) and the response is
`200 text/html`; a render failure or uninitialised engine yields a `500`.
Layouts are applied the Tera way — the view does
`{% extends "layouts/application.html.tera" %}` — which is what the scaffold
generator emits. (`doido_view::Renderer` additionally offers the
`content_for_layout` style for apps that prefer controller-chosen layouts.)
Custom engines: install any `TemplateEngine` with `doido_view::set_engine(...)`.

```rust
// ctx.render delegates to doido-view
ctx.render("posts/index", json!({ "posts": posts }))

// with explicit status
ctx.render("posts/new", json!({ "post": post })).status(422)

// JSON response (skips template engine entirely)
ctx.json(json!({ "posts": posts }))

// layout override
ctx.render("posts/index", data).layout("admin")

// no layout
ctx.render("posts/index", data).no_layout()
```

## Config

```toml
[view]
engine = "tera"           # default
templates_dir = "views"   # relative to app root
layout = "application"    # default layout name
hot_reload = true         # dev only — watch templates dir for changes
```

## Open Questions (remaining)

- [ ] View helpers (like Rails `link_to`, `form_for`) — Tera custom functions/filters or Rust fns injected into context?
- [ ] Content negotiation (HTML vs JSON) — controller-driven or automatic via `Accept` header?

## Known Requirements

- `TemplateEngine` trait — swappable
- `TeraEngine` ships as default impl
- Template resolution by convention (`views/<controller>/<action>.html.tera`)
- Layout system with `{{ content_for_layout }}`
- Partial support via Tera `{% include %}`
- Hot reload in development via `reload()` on file change
- JSON responses bypass template engine entirely
- Engine configured in `doido-config`

## TDD Surface

- Test `TeraEngine::render` produces correct HTML with given context
- Test unknown template returns clear error (not panic)
- Test layout wraps rendered template content correctly
- Test `no_layout()` skips layout
- Test custom engine implementing `TemplateEngine` trait works as drop-in
- Test hot reload picks up template changes without restart
- Integration test: controller `ctx.render(...)` → full HTML response via test client
- Integration test: controller `ctx.json(...)` → JSON response, no template involved
