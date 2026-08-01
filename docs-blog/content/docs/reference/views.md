+++
title = "Views"
description = "Tera templates, layouts, partials, view helpers, fragment caching, and swappable engines."
weight = 6
aliases = ['/docs/guides/views/']

+++

> **Design spec:** [`docs/04-view.md`](https://github.com/doido-rs/doido/blob/master/docs/04-view.md).
> This guide documents the API as implemented in `doido-view`.

**Rails analogue: Action View.** Views render HTML from [Tera](https://keats.github.io/tera/)
templates by convention, wrapped in layouts. The engine is swappable behind the
`TemplateEngine` trait, and the crate ships Rails-style view helpers, partials, `content_for`
blocks, and fragment caching.

## At a glance

```rust
use doido::view::{init, render, set_engine, TemplateEngine, TeraEngine, ViewResponse};
use doido::view::{render_partial, render_collection};
```

## Setup

Install the global engine once at boot, pointing at your templates directory.

```rust
doido::view::init("app/views")?; // load all *.html.tera under app/views
```

## Rendering from controllers

Actions render through [`ctx.render(...)`](@/docs/reference/controllers.md), which delegates to
the global engine. The template name follows the convention
`views/<controller>/<action>.html.tera`, and the JSON value you pass becomes the template
context.

```rust
async fn index(ctx: Context) -> Response {
    ctx.render("posts/index", json!({ "posts": posts }))
}
```

```html
{# app/views/posts/index.html.tera #}
{% for post in posts %}
  <article><h2>{{ post.title }}</h2><p>{{ post.body }}</p></article>
{% endfor %}
```

## Layouts

Rendered content is wrapped in `layouts/application.html.tera` by default, which yields to
the page with `{{ content_for_layout }}`. Override or skip the layout per render with
`ViewResponse::layout(...)` / `no_layout()`.

```html
{# app/views/layouts/application.html.tera #}
<!DOCTYPE html>
<html><body>{{ content_for_layout }}</body></html>
```

```rust
use doido::view::ViewResponse;

ViewResponse::new("posts/index", json!({}));               // default layout
ViewResponse::new("posts/index", json!({})).layout("admin"); // layouts/admin.html.tera
ViewResponse::new("email/welcome", json!({})).no_layout();   // raw content
ViewResponse::new("posts/new", json!({})).status(422);       // set the status
```

## Partials

Reuse fragments with Tera's `{% include %}`, or render one directly. `render_collection`
renders a partial once per item, binding each to a variable name.

```rust
use doido::view::{render_partial, render_collection};

let html = render_partial("shared/_card", &json!({ "title": "Hi" }))?;
let list = render_collection("posts/_post", &posts, "post")?; // one render per post
```

```html
{% include "shared/_header.html.tera" %}
```

## content_for blocks

Capture named content in a template (e.g. a page title or extra `<head>` tags) and yield it
elsewhere in the layout with `ContentFor`.

```rust
use doido::view::ContentFor;

let mut content = ContentFor::new();
content.set("title", "Dashboard");
let title = content.get("title"); // "Dashboard"
```

## View helpers

`doido::view::helpers` provides Rails-style HTML helpers that return escaped HTML strings:

```rust
use doido::view::helpers::{link, form, asset, number, sanitize, tag, hotwire};

link::link_to("Home", "/");                       // <a href="/">Home</a>
link::button_to("Delete", "/posts/1", "delete");  // form-wrapped button
form::text_field("title", "Hello");               // <input …>
form::submit("Save");
asset::image_tag("logo.png");
asset::stylesheet_link_tag("application");
number::number_to_currency(1999.0);               // "$1,999.00"
number::number_with_delimiter(1000000);           // "1,000,000"
sanitize::strip_tags("<b>hi</b>");                // "hi"
tag::content_tag("span", "hi", &[("class", "muted")]);
hotwire::turbo_frame("messages", "…");            // Turbo frame
```

## Fragment caching

`cache_fragment` returns cached HTML for a key, running the render closure only on a miss —
backed by any [cache store](@/docs/reference/cache.md).

```rust
use doido::view::fragment::cache_fragment;

let html = cache_fragment(&cache_store, "posts/1", || {
    render_partial("posts/_post", &post).unwrap()
}).await;
```

## Swappable engine

Tera is the default, but any type implementing `TemplateEngine` (`render` + `reload`) can
replace it via `set_engine`.

```rust
use doido::view::{TemplateEngine, set_engine};
use std::sync::Arc;

struct MyEngine;
impl TemplateEngine for MyEngine {
    fn render(&self, template: &str, ctx: &serde_json::Value) -> doido::Result<String> {
        Ok(format!("rendered:{template}"))
    }
    fn reload(&self) -> doido::Result<()> { Ok(()) }
}

set_engine(Arc::new(MyEngine));
```

`TeraEngine` reloads templates from disk on `reload()`, which the framework calls in
development for hot reloading.

## See also

- [Controllers & routing](@/docs/reference/controllers.md) — `ctx.render(...)` and content negotiation.
- [Cache](@/docs/reference/cache.md) — the store behind fragment caching.
- [Mailer](@/docs/reference/mailer.md) — reuses this engine for email templates.
