+++
title = "Design specifications"
description = "The per-crate design specs in the repository — source of truth for design intent."
weight = 1
+++

This manual is curated for **day-to-day usage**. When you need the full design
intent — interview decisions, API contracts before implementation, reconciliation
notes — read the specs in the repository.

## Spec index

| Spec | Crate | Topic |
|------|-------|-------|
| [`00-overview.md`](https://github.com/doido-rs/doido/blob/master/docs/00-overview.md) | all | Philosophy, crate map, TDD strategy |
| [`01-router.md`](https://github.com/doido-rs/doido/blob/master/docs/01-router.md) | `doido-controller` | Route DSL, URL helpers |
| [`02-controller.md`](https://github.com/doido-rs/doido/blob/master/docs/02-controller.md) | `doido-controller` | Controllers, params, filters |
| [`03-model.md`](https://github.com/doido-rs/doido/blob/master/docs/03-model.md) | `doido-model` | sea-orm, connection pool |
| [`04-view.md`](https://github.com/doido-rs/doido/blob/master/docs/04-view.md) | `doido-view` | Templates, layouts, engines |
| [`05-config.md`](https://github.com/doido-rs/doido/blob/master/docs/05-config.md) | `doido-config` | YAML config, credentials |
| [`06-cli.md`](https://github.com/doido-rs/doido/blob/master/docs/06-cli.md) | `doido-generators` | Runtime CLI commands |
| [`06b-generators.md`](https://github.com/doido-rs/doido/blob/master/docs/06b-generators.md) | `doido-generators` | Code generators |
| [`07-middleware.md`](https://github.com/doido-rs/doido/blob/master/docs/07-middleware.md) | `doido-controller` | Middleware stack, sessions |
| [`08-mailer.md`](https://github.com/doido-rs/doido/blob/master/docs/08-mailer.md) | `doido-mailer` | Email composition, delivery |
| [`09-jobs.md`](https://github.com/doido-rs/doido/blob/master/docs/09-jobs.md) | `doido-jobs` | Background jobs, queues |
| [`10-cache.md`](https://github.com/doido-rs/doido/blob/master/docs/10-cache.md) | `doido-cache` | Cache stores |
| [`11-core.md`](https://github.com/doido-rs/doido/blob/master/docs/11-core.md) | `doido-core` | Errors, inflector, utilities |
| [`12-cable.md`](https://github.com/doido-rs/doido/blob/master/docs/12-cable.md) | `doido-cable` | WebSocket channels |
| [`15-storage.md`](https://github.com/doido-rs/doido/blob/master/docs/15-storage.md) | `doido-storage` | Attached-file storage |

## Authoritative architecture doc

For what is **actually built** — crate status, config reconciliation, and the
runtime boot sequence — see
[`docs/ARCHITECTURE.md`](https://github.com/doido-rs/doido/blob/master/docs/ARCHITECTURE.md)
in the repository.

Each **[Reference](/docs/reference/)** guide links to its spec at the top of the page.
