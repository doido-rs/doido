+++
title = "CLI & generators"
description = "Every runtime command and code generator in the doido binary."
weight = 3
+++

The `doido` binary is the single entry point for both runtime commands and code
generation.

## Runtime commands

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

## Generators

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
| `doido-storage` | Active Storage | Attached-file storage (disk / S3 / R2 / Azure) |

For the full design intent of each crate, see the
[spec documents](https://github.com/doido-rs/doido/tree/master/docs).
