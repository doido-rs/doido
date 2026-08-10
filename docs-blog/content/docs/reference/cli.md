+++
title = "CLI & generators"
description = "Every runtime command and code generator in the doido binary."
weight = 1
aliases = ["/docs/cli/"]
+++

The global `doido` CLI creates new applications. Inside a generated project,
`cargo doido` (a Cargo alias set up by `doido new`) is the entry point for
runtime commands and code generation.

## Runtime commands

| Command | Description |
|---------|-------------|
| `doido new <name>` | Create a new application (`--database=sqlite\|postgres\|mysql`) |
| `cargo doido server` | Start the web server |
| `cargo doido routes` | Print the route table |
| `cargo doido console` | Start an interactive console |
| `cargo doido db <cmd>` | Create databases, run SeaORM migrations, seed fixture data (`db/seed`), generate entities |
| `cargo doido jobs <cmd>` | Inspect and manage background jobs |
| `cargo doido worker` | Run the background job worker (`--once` to drain and exit) |
| `cargo doido credentials <cmd>` | Manage AES-256-GCM encrypted credentials |
| `cargo doido generate <gen>` | Run a code generator (see below) |

## Generators

Run `cargo doido generate` with no arguments to list every registered generator:

| Generator | Generates |
|-----------|-----------|
| `controller` | A controller with actions |
| `helper` | A controller helper in `app/helpers/` |
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
| `doido-controller` | Action Dispatch + Controller + Rack | Route DSL, request handling, params, controller helpers, Tower middleware, sessions |
| `doido-model` | Active Record | sea-orm re-exports, connection pool, test helpers |
| `doido-view` | Action View | Tera templates, layouts, partials |
| `doido-config` | Rails `config/` | Layered TOML/YAML config, encrypted credentials, env overrides |
| `doido-generators` | `rails` CLI + generators | Runtime commands and code generators |
| `doido-mailer` | Action Mailer | Email composition and delivery |
| `doido-jobs` | Active Job | Background jobs with pluggable backends and retries |
| `doido-cache` | Active Support Cache | Pluggable cache store (memory / redis / memcache) |
| `doido-cable` | Action Cable | WebSocket channels and pub/sub |
| `doido-storage` | Active Storage | Attached-file storage (disk / S3 / R2 / Azure) |

For the full design intent of each crate, see the
[spec documents](https://github.com/doido-rs/doido/tree/master/docs).
