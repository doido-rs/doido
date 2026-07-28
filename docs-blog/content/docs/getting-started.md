+++
title = "Getting started"
description = "Create, run, and understand your first Doido application."
weight = 1
+++

Doido follows Rails-style conventions: a single `doido` binary scaffolds an
application, runs the server, manages the database, and drives code generators.

## Create an application

```bash
# Create a new application (sqlite by default; --database=postgres|mysql)
doido new blog
cd blog

# Set up the database and run pending migrations
doido db create
doido db migrate

# Boot the HTTP server on http://0.0.0.0:3000
doido server
```

`GET /` answers with JSON from the generated `HelloController`:

```json
{ "message": "Hello word!" }
```

## A taste of the code

A controller is a plain `impl` block annotated with `#[controller]`:

```rust
use doido_controller::controller;
use serde_json::json;

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(ctx: doido_controller::Context) -> doido_controller::Response {
        ctx.json(json!({ "message": "Hello word!" }))
    }
}
```

Routes are declared with the `routes!` macro in `config/routes.rs`:

```rust
use crate::controllers::HelloController;
use doido::controller::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        get!("/", HelloController::index);
        // resources!(PostsController);   // all 7 REST routes
    }
}
```

## Project layout

A generated application follows Rails-style conventions:

```
my-app/
├── Cargo.toml
├── src/main.rs              ← delegates to doido::run(routes)
├── config/
│   ├── application.toml      ← base config
│   ├── development.yml       ← per-environment overrides
│   ├── test.yml
│   ├── production.yml
│   ├── routes.rs            ← routes! macro
│   └── inflection.yaml      ← custom pluralization rules
├── app/
│   ├── controllers/
│   ├── models/
│   └── views/
├── db/
│   ├── migration/           ← SeaORM migration crate
│   └── schema/
└── tests/
```

## Configuration

Configuration is layered: `config/application.toml` provides the base, then
`config/<env>.yml` (development / test / production) overrides per environment.
Encrypted credentials and `SECTION__KEY` environment variables override on top.

```yaml
# config/development.yml
server:
  bind: 0.0.0.0
  port: 3000
database:
  url: sqlite://db/development.db
logger:
  level: debug
  format: verbose
cache:
  type: memory
```

## Next steps

- **[Installation](@/docs/installation.md)** — prerequisites and how to install the CLI.
- **[CLI & generators](@/docs/cli.md)** — every runtime command and code generator.
- **[Controllers & routing](@/docs/guides/controllers.md)** — the request/response guide.
