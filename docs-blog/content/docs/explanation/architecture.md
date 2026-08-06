+++
title = "Architecture"
description = "How Doido is structured — crate map, boot sequence, and design principles."
weight = 1
+++

> **Design spec:** [`docs/00-overview.md`](https://github.com/doido-rs/doido/blob/master/docs/00-overview.md)
> and [`docs/ARCHITECTURE.md`](https://github.com/doido-rs/doido/blob/master/docs/ARCHITECTURE.md).

Doido is a **workspace of focused crates** rather than a monolith. Each subsystem
mirrors a Rails component and can be used on its own, but the generators and CLI
wire them together into a conventional application layout.

## Crate map

| Crate | Rails analogue | Responsibility |
|-------|----------------|----------------|
| `doido-core` | Active Support | Errors, inflector, shared utilities |
| `doido-config` | — | Per-env YAML, encrypted credentials |
| `doido-controller` | Action Dispatch + Controller | Routes, controllers, controller helpers, middleware |
| `doido-auth` | Devise + OmniAuth + JWT | AuthUser trait, strategies, extractors, generators |
| `doido-model` | Active Record | sea-orm re-exports, pool, test helpers |
| `doido-view` | Action View | Tera templates, layouts, helpers |
| `doido-generators` | Rails generators + CLI | Scaffolds, `cargo doido server`, `cargo doido db` |
| `doido-mailer` | Action Mailer | Email composition and delivery |
| `doido-jobs` | Active Job | Background jobs and queues |
| `doido-cache` | Active Support Cache | Pluggable cache stores |
| `doido-cable` | Action Cable | WebSocket channels and pub/sub |
| `doido-storage` | Active Storage | Attached files and blob storage |

## Design principles

**Convention over configuration.** Routes live in `config/routes.rs`, templates in
`views/`, models in `models/`, controller helpers in `helpers/` — the generators
scaffold the layout so you spend time on business logic, not wiring.

**TDD-first specs.** Every crate has a design spec under `docs/` in the repository.
This manual is the curated, hand-written layer on top; the specs are the source of
truth for design intent.

**Async-native stack.** Controllers are `async fn` handlers on axum. The ORM is
sea-orm. Nothing blocks the runtime unless you ask it to.

**Pluggable backends.** Jobs, cache, mail delivery, storage services, cable pub/sub,
session stores, and auth strategies all accept swappable backends — memory and SQLite
defaults for local development, Redis/Postgres/S3 in production.

## Boot sequence

1. Load `config/<env>.yml` (with optional credential decryption).
2. Connect the database pool (`doido-model`).
3. Initialise auth when enabled (`doido-auth::init`).
4. Build the axum router from `config/routes.rs` (`doido-controller`).
5. Layer middleware (logging, sessions, auth, CORS, …) from config.
6. Bind and serve (`cargo doido server`).

See **[Getting started](/docs/tutorials/getting-started/)** to walk through this
in a real app, or browse the **[Reference](/docs/reference/)** for subsystem APIs.
