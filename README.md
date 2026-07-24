# Doido

A Rails-inspired web framework for Rust, built on [axum](https://github.com/tokio-rs/axum)
and [sea-orm](https://www.sea-ql.org/SeaORM/).

Doido brings the productivity of Rails — convention over configuration, a rich
generator-driven CLI, and batteries-included subsystems — to an async-native,
strongly-typed Rust stack. Every subsystem lives in its own crate and can be used
on its own.

> **Status:** early development (`0.0.x`). APIs are not yet stable.

## Philosophy

- **Convention over configuration** — sensible defaults, opt-in overrides.
- **Batteries included** — routing, ORM, views, mailer, jobs, cache, websockets out of the box.
- **Modular** — each crate is independently usable and testable.
- **TDD-first** — every module ships with test helpers and a well-defined test surface.
- **Async-native** — Tokio + async/await throughout; no sync shims.

## Quick start

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

## CLI

The `doido` binary is the single entry point for both runtime commands and code generation:

| Command | Description |
|---------|-------------|
| `doido new <name>` | Create a new application (`--database=sqlite\|postgres\|mysql`) |
| `doido server` | Start the web server |
| `doido routes` | Print the route table |
| `doido console` | Start an interactive console |
| `doido db <cmd>` | Create databases, run SeaORM migrations, generate entities |
| `doido jobs <cmd>` | Inspect and manage background jobs |
| `doido worker` | Run the background job worker (`--once` to drain and exit) |
| `doido credentials <cmd>` | Manage AES-256-GCM encrypted credentials |
| `doido generate <gen>` | Run a code generator (see below) |

### Generators

Run `doido generate` with no arguments to list every registered generator:

| Generator | Generates |
|-----------|-----------|
| `controller` | A controller with actions |
| `model` | A model + SeaORM migration |
| `migration` | A standalone migration |
| `scaffold` | Model, controller, views, routes — the full CRUD stack |
| `job` | A background job |
| `mailer` | A mailer with templates |
| `channel` | A WebSocket channel |
| `templates` | View templates for an existing controller |
| `generator` | A new custom generator (the registry is extensible) |

Generators auto-inject routes into `config/routes.rs` and honor custom
pluralization rules declared in `config/inflection.yaml`.

## Workspace crates

| Crate | Rails analogue | Responsibility |
|-------|----------------|----------------|
| `doido` | `rails` binary | Entry point, app runtime |
| `doido-core` | Active Support | Shared traits, errors, inflector, logger, utilities |
| `doido-controller` | Action Dispatch + Controller + Rack | Route DSL, request handling, params, Tower middleware, sessions |
| `doido-model` | Active Record | sea-orm re-exports, connection pool, test helpers |
| `doido-view` | Action View | Tera templates, layouts, partials |
| `doido-config` | Rails `config/` | Layered TOML/YAML config, encrypted credentials, env overrides |
| `doido-generators` | `rails` CLI + generators | Runtime commands and code generators |
| `doido-mailer` | Action Mailer | Email composition and delivery |
| `doido-jobs` | Active Job | Background jobs with pluggable backends and retries |
| `doido-cache` | Active Support Cache | Pluggable cache store (memory / redis / memcache) |
| `doido-cable` | Action Cable | WebSocket channels and pub/sub |

> `doido-kafka` and `doido-mcp` are specified in [`docs/`](docs/) and planned.

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

## Development

```bash
cargo build        # build the whole workspace
cargo test         # run all tests (TDD-first)
cargo fmt          # format
cargo clippy       # lint
make check         # cargo-deny supply-chain checks
```

The spec documents that drive the design live in [`docs/`](docs/), indexed from
[`CLAUDE.md`](CLAUDE.md).

## License

MIT
