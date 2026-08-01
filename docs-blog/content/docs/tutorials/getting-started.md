+++
title = "Getting started"
description = "Create, run, and understand your first Doido application."
weight = 1
aliases = ["/docs/getting-started/"]
+++

Doido follows Rails-style conventions: the global `doido` CLI scaffolds new
applications; inside a project, `cargo doido` runs the server, manages the
database, and drives code generators.

## Create an application

```bash
# Create a new application (sqlite by default; --database=postgres|mysql)
doido new blog
cd blog

# Set up the database and run pending migrations
cargo doido db create
cargo doido db migrate

# Boot the HTTP server on http://0.0.0.0:3000
cargo doido server
```

`GET /` answers with JSON from the generated `HelloController`:

```json
{ "message": "Hello word!" }
```

## A taste of the code

A controller is a plain `impl` block annotated with `#[controller]`:

```rust
use doido::controller::{controller, Context, Response};
use serde_json::json;

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(ctx: Context) -> Response {
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

- **[Installation](@/docs/setup/installation.md)** — prerequisites and how to install the CLI.
- **[CLI & generators](@/docs/reference/cli.md)** — every runtime command and code generator.
- **[Controllers & routing](@/docs/reference/controllers.md)** — the request/response guide.
