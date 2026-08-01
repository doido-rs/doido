+++
title = "Views"
description = "Templates Tera, layouts, partials, view helpers, cache de fragmento e engines substituíveis."
weight = 6
+++

> **Especificação de design:** [`docs/04-view.md`](https://github.com/doido-rs/doido/blob/master/docs/04-view.md).
> Este guia documenta a API como implementada em `doido-view`.

**Análogo no Rails: Action View.** As views renderizam HTML a partir de templates
[Tera](https://keats.github.io/tera/) por convenção, envolvidos em layouts. A engine é
substituível por trás do trait `TemplateEngine`, e o crate traz view helpers no estilo
Rails, partials, blocos `content_for` e cache de fragmento.

## Visão geral

```rust
use doido::view::{init, render, set_engine, TemplateEngine, TeraEngine, ViewResponse};
use doido::view::{render_partial, render_collection};
```

## Configuração

Instale a engine global uma vez no boot, apontando para o seu diretório de templates.

```rust
doido::view::init("app/views")?; // carrega todos os *.html.tera sob app/views
```

## Renderizando a partir de controllers

As actions renderizam via [`ctx.render(...)`](@/docs/reference/controllers.pt.md), que delega
para a engine global. O nome do template segue a convenção
`views/<controller>/<action>.html.tera`, e o valor JSON que você passa vira o contexto do
template.

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

O conteúdo renderizado é envolvido em `layouts/application.html.tera` por padrão, que cede
o lugar à página com `{{ content_for_layout }}`. Sobrescreva ou pule o layout por render
com `ViewResponse::layout(...)` / `no_layout()`.

```html
{# app/views/layouts/application.html.tera #}
<!DOCTYPE html>
<html><body>{{ content_for_layout }}</body></html>
```

```rust
use doido::view::ViewResponse;

ViewResponse::new("posts/index", json!({}));               // layout padrão
ViewResponse::new("posts/index", json!({})).layout("admin"); // layouts/admin.html.tera
ViewResponse::new("email/welcome", json!({})).no_layout();   // conteúdo cru
ViewResponse::new("posts/new", json!({})).status(422);       // define o status
```

## Partials

Reutilize fragmentos com o `{% include %}` do Tera, ou renderize um diretamente.
`render_collection` renderiza um partial uma vez por item, ligando cada um a um nome de
variável.

```rust
use doido::view::{render_partial, render_collection};

let html = render_partial("shared/_card", &json!({ "title": "Hi" }))?;
let list = render_collection("posts/_post", &posts, "post")?; // um render por post
```

```html
{% include "shared/_header.html.tera" %}
```

## Blocos content_for

Capture conteúdo nomeado em um template (ex.: um título de página ou tags `<head>` extras)
e ceda-o em outro lugar do layout com `ContentFor`.

```rust
use doido::view::ContentFor;

let mut content = ContentFor::new();
content.set("title", "Dashboard");
let title = content.get("title"); // "Dashboard"
```

## View helpers

`doido::view::helpers` fornece helpers HTML no estilo Rails que retornam strings HTML
escapadas:

```rust
use doido::view::helpers::{link, form, asset, number, sanitize, tag, hotwire};

link::link_to("Home", "/");                       // <a href="/">Home</a>
link::button_to("Delete", "/posts/1", "delete");  // botão dentro de um form
form::text_field("title", "Hello");               // <input …>
form::submit("Save");
asset::image_tag("logo.png");
asset::stylesheet_link_tag("application");
number::number_to_currency(1999.0);               // "$1,999.00"
number::number_with_delimiter(1000000);           // "1,000,000"
sanitize::strip_tags("<b>hi</b>");                // "hi"
tag::content_tag("span", "hi", &[("class", "muted")]);
hotwire::turbo_frame("messages", "…");            // frame do Turbo
```

## Cache de fragmento

`cache_fragment` retorna o HTML cacheado para uma chave, rodando a closure de render apenas
em um miss — apoiado por qualquer [cache store](@/docs/reference/cache.pt.md).

```rust
use doido::view::fragment::cache_fragment;

let html = cache_fragment(&cache_store, "posts/1", || {
    render_partial("posts/_post", &post).unwrap()
}).await;
```

## Engine substituível

O Tera é o padrão, mas qualquer tipo que implemente `TemplateEngine` (`render` + `reload`)
pode substituí-lo via `set_engine`.

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

`TeraEngine` recarrega os templates do disco no `reload()`, que o framework chama em
desenvolvimento para hot reload.

## Veja também

- [Controllers & roteamento](@/docs/reference/controllers.pt.md) — `ctx.render(...)` e negociação de conteúdo.
- [Cache](@/docs/reference/cache.pt.md) — o store por trás do cache de fragmento.
- [Mailer](@/docs/reference/mailer.pt.md) — reutiliza esta engine para templates de e-mail.
