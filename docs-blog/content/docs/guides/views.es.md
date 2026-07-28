+++
title = "Vistas"
description = "Plantillas Tera, layouts, partials, view helpers, caché de fragmentos y engines intercambiables."
weight = 6
+++

> **Especificación de diseño:** [`docs/04-view.md`](https://github.com/doido-rs/doido/blob/master/docs/04-view.md).
> Esta guía documenta la API tal como está implementada en `doido-view`.

**Análogo en Rails: Action View.** Las vistas renderizan HTML a partir de plantillas
[Tera](https://keats.github.io/tera/) por convención, envueltas en layouts. El engine es
intercambiable detrás del trait `TemplateEngine`, y el crate incluye view helpers al estilo
Rails, partials, bloques `content_for` y caché de fragmentos.

## Vistazo general

```rust
use doido_view::{init, render, set_engine, TemplateEngine, TeraEngine, ViewResponse};
use doido_view::{render_partial, render_collection};
```

## Configuración

Instala el engine global una vez en el arranque, apuntando a tu directorio de plantillas.

```rust
doido_view::init("app/views")?; // carga todos los *.html.tera bajo app/views
```

## Renderizar desde los controladores

Las actions renderizan vía [`ctx.render(...)`](@/docs/guides/controllers.es.md), que delega
al engine global. El nombre de la plantilla sigue la convención
`views/<controller>/<action>.html.tera`, y el valor JSON que pasas se convierte en el
contexto de la plantilla.

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

El contenido renderizado se envuelve en `layouts/application.html.tera` por defecto, que
cede el lugar a la página con `{{ content_for_layout }}`. Sobrescribe o salta el layout por
render con `ViewResponse::layout(...)` / `no_layout()`.

```html
{# app/views/layouts/application.html.tera #}
<!DOCTYPE html>
<html><body>{{ content_for_layout }}</body></html>
```

```rust
use doido_view::ViewResponse;

ViewResponse::new("posts/index", json!({}));               // layout por defecto
ViewResponse::new("posts/index", json!({})).layout("admin"); // layouts/admin.html.tera
ViewResponse::new("email/welcome", json!({})).no_layout();   // contenido crudo
ViewResponse::new("posts/new", json!({})).status(422);       // define el estado
```

## Partials

Reutiliza fragmentos con el `{% include %}` de Tera, o renderiza uno directamente.
`render_collection` renderiza un partial una vez por ítem, ligando cada uno a un nombre de
variable.

```rust
use doido_view::{render_partial, render_collection};

let html = render_partial("shared/_card", &json!({ "title": "Hi" }))?;
let list = render_collection("posts/_post", &posts, "post")?; // un render por post
```

```html
{% include "shared/_header.html.tera" %}
```

## Bloques content_for

Captura contenido con nombre en una plantilla (p. ej. un título de página o etiquetas
`<head>` extra) y cédelo en otro lugar del layout con `ContentFor`.

```rust
use doido_view::ContentFor;

let mut content = ContentFor::new();
content.set("title", "Dashboard");
let title = content.get("title"); // "Dashboard"
```

## View helpers

`doido_view::helpers` provee helpers HTML al estilo Rails que devuelven cadenas HTML
escapadas:

```rust
use doido_view::helpers::{link, form, asset, number, sanitize, tag, hotwire};

link::link_to("Home", "/");                       // <a href="/">Home</a>
link::button_to("Delete", "/posts/1", "delete");  // botón dentro de un form
form::text_field("title", "Hello");               // <input …>
form::submit("Save");
asset::image_tag("logo.png");
asset::stylesheet_link_tag("application");
number::number_to_currency(1999.0);               // "$1,999.00"
number::number_with_delimiter(1000000);           // "1,000,000"
sanitize::strip_tags("<b>hi</b>");                // "hi"
tag::content_tag("span", "hi", &[("class", "muted")]);
hotwire::turbo_frame("messages", "…");            // frame de Turbo
```

## Caché de fragmentos

`cache_fragment` devuelve el HTML cacheado para una clave, ejecutando la closure de render
solo en un miss — respaldado por cualquier [cache store](@/docs/guides/cache.es.md).

```rust
use doido_view::fragment::cache_fragment;

let html = cache_fragment(&cache_store, "posts/1", || {
    render_partial("posts/_post", &post).unwrap()
}).await;
```

## Engine intercambiable

Tera es el engine por defecto, pero cualquier tipo que implemente `TemplateEngine`
(`render` + `reload`) puede reemplazarlo vía `set_engine`.

```rust
use doido_view::{TemplateEngine, set_engine};
use std::sync::Arc;

struct MyEngine;
impl TemplateEngine for MyEngine {
    fn render(&self, template: &str, ctx: &serde_json::Value) -> doido_core::Result<String> {
        Ok(format!("rendered:{template}"))
    }
    fn reload(&self) -> doido_core::Result<()> { Ok(()) }
}

set_engine(Arc::new(MyEngine));
```

`TeraEngine` recarga las plantillas desde el disco en `reload()`, que el framework llama en
desarrollo para hot reload.

## Véase también

- [Controladores y enrutamiento](@/docs/guides/controllers.es.md) — `ctx.render(...)` y negociación de contenido.
- [Cache](@/docs/guides/cache.es.md) — el store detrás de la caché de fragmentos.
- [Mailer](@/docs/guides/mailer.es.md) — reutiliza este engine para las plantillas de correo.
