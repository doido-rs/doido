# Doido Framework — Context Index

Doido is a Rails-inspired web framework in Rust (axum + sea-orm).
Implementation is TDD-first. No code exists yet — this CLAUDE.md is a pointer index
to the spec documents that drive the interview and planning process.

## Spec Documents

| File | Crate | Description |
|------|-------|-------------|
| [docs/00-overview.md](docs/00-overview.md) | all | Framework philosophy, crate map, TDD strategy |
| [docs/01-router.md](docs/01-router.md) | `doido-controller` | Route DSL, URL helpers, Action Dispatch analogue (merged into `doido-controller`) |
| [docs/02-controller.md](docs/02-controller.md) | `doido-controller` | Request handling, params, filters, Action Controller analogue |
| [docs/03-model.md](docs/03-model.md) | `doido-model` | sea-orm re-exports + connection pool + test helpers |
| [docs/04-view.md](docs/04-view.md) | `doido-view` | Tera template engine, layouts, partials, Action View analogue |
| [docs/05-config.md](docs/05-config.md) | `doido-config` | TOML layered config, encrypted credentials, env var overrides |
| [docs/06-cli.md](docs/06-cli.md) | `doido-generators` | CLI runtime commands (server, console, db, worker, credentials) — merged into `doido-generators` |
| [docs/06b-generators.md](docs/06b-generators.md) | `doido-generators` | All Rails generator targets + the unified CLI, extensible registry, route auto-injection |
| [docs/07-middleware.md](docs/07-middleware.md) | `doido-controller` | Tower middleware stack, sessions, CORS, Rack analogue — merged into `doido-controller` |
| [docs/08-mailer.md](docs/08-mailer.md) | `doido-mailer` | Email composition, delivery backends, Action Mailer analogue |
| [docs/09-jobs.md](docs/09-jobs.md) | `doido-jobs` | Background jobs, queue backends, Active Job analogue |
| [docs/10-cache.md](docs/10-cache.md) | `doido-cache` | Pluggable cache store, TTL, Active Support Cache analogue |
| [docs/11-core.md](docs/11-core.md) | `doido-core` | Shared errors, inflector, utilities, Active Support analogue |
| [docs/12-cable.md](docs/12-cable.md) | `doido-cable` | WebSocket channels, pub/sub, Action Cable analogue |
| [docs/13-kafka.md](docs/13-kafka.md) | `doido-kafka` | Kafka producers and consumers, messaging integration |
| [docs/14-mcp.md](docs/14-mcp.md) | `doido-mcp` | MCP server (tools/resources) + MCP client integration |

## Workspace Layout

```
doido/                  ← workspace root (Cargo.toml)
├── doido/              ← binary entry point
├── doido-core/         ← shared traits, errors, utilities
├── doido-controller/   ← action controller + route DSL (routes! macro) + tower middleware/sessions on axum
├── doido-model/        ← sea-orm re-exports + framework glue
├── doido-view/         ← templates and response helpers
├── doido-config/       ← environment config
├── doido-generators/   ← code generators (model, scaffold, job…) + CLI (server, db, worker…)
├── doido-mailer/       ← email
├── doido-jobs/         ← background jobs
├── doido-cache/        ← cache store
├── doido-cable/        ← websocket channels + pub/sub
├── doido-kafka/        ← kafka producers + consumers
└── doido-mcp/          ← mcp server + client
```

## Interview Status

- [x] 01-router — **Macro DSL, `resources!` with all 7 REST routes, `only:`/`except:`, namespace/scope**
- [x] 02-controller — **`#[controller]` macro + trait, `#[before_action]`/`#[after_action]` attrs, Tower middleware layers**
- [x] 03-model — **Re-exports sea-orm fully, adds only connection pool + test helpers (SQLite in-memory)**
- [x] 04-view — **Tera default engine, swappable via `TemplateEngine` trait, convention-based template resolution**
- [x] 05-config — **TOML, layered (base→env→credentials→env vars), AES-256-GCM encrypted credentials, `SECTION__KEY` env override**
- [x] 06-cli — **Runtime commands only; `doido generate` delegates to `doido-generators`**
- [x] 06b-generators — **Separate crate, all Rails targets, `Generator` trait registry, auto-injects `config/routes.rs`**
- [x] 07-middleware — **Logging+PanicRecovery always-on, all else opt-in via config, pluggable `SessionStore` (cookie default)**
- [x] 08-mailer — **`deliver_now()` + `deliver_later()`, templates in `views/mailers/`, pluggable `Deliverer` trait**
- [x] 09-jobs — **Pluggable backends (memory/db/redis), exponential retry per-job via `#[job]` macro, dead letter queue + CLI**
- [x] 10-cache — **Pluggable backends (memory/redis/db), configurable namespacing (`app:env:custom:key`), multiple named stores**
- [x] 11-core — **`thiserror` per crate + `anyhow` at app level, all inflections + `config/inflections.rs` for custom rules**
- [x] 12-cable — **`#[channel]` macro + trait, pluggable PubSub (memory/redis/db), middleware+`CableConnection` auth, ActionCable wire protocol, generator added**
- [x] 13-kafka — **Kafka-specific opt-in crate, `rskafka`, `#[consumer]` + `#[topic]`, pluggable `MessageCodec`, dispatch-to-jobs pattern**
- [x] 14-mcp — **HTTP+SSE transport, `#[tool]` on fns, `#[resource]`+`#[mcp_resource]` on models, raw+typed client, middleware+OAuth2.1 auth**
