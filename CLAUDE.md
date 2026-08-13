# Doido Framework — Context Index

Doido is a Rails-inspired web framework in Rust (axum + sea-orm).
Implementation is TDD-first. The table below indexes the **spec** documents (design
intent). For what is **actually built** — the crate ↔ spec status, reconciliation
decisions (config), and the runtime boot sequence — see the authoritative
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Green gate: `make verify`.

## Spec Documents

| File | Crate | Description |
|------|-------|-------------|
| [docs/00-overview.md](docs/00-overview.md) | all | Framework philosophy, crate map, TDD strategy |
| [docs/01-router.md](docs/01-router.md) | `doido-controller` | Route DSL, URL helpers, Action Dispatch analogue (merged into `doido-controller`) |
| [docs/02-controller.md](docs/02-controller.md) | `doido-controller` | Request handling, params, filters, Action Controller analogue |
| [docs/03-model.md](docs/03-model.md) | `doido-model` | sea-orm re-exports + connection pool + test helpers |
| [docs/04-view.md](docs/04-view.md) | `doido-view` | Tera template engine, layouts, partials, Action View analogue |
| [docs/05-config.md](docs/05-config.md) | `doido-config` | Per-env YAML config, AES-256-GCM encrypted credentials, env var overrides |
| [docs/06-cli.md](docs/06-cli.md) | `doido-generators` | CLI runtime commands (server, console, db, worker, credentials) — merged into `doido-generators` |
| [docs/06b-generators.md](docs/06b-generators.md) | `doido-generators` | All Rails generator targets + the unified CLI, extensible registry, route auto-injection |
| [docs/07-middleware.md](docs/07-middleware.md) | `doido-controller` | Tower middleware stack, sessions, CORS, Rack analogue — merged into `doido-controller` |
| [docs/08-mailer.md](docs/08-mailer.md) | `doido-mailer` | Email composition, delivery backends, Action Mailer analogue |
| [docs/09-jobs.md](docs/09-jobs.md) | `doido-jobs` | Background jobs, queue backends, Active Job analogue |
| [docs/10-cache.md](docs/10-cache.md) | `doido-cache` | Pluggable cache store, TTL, Active Support Cache analogue |
| [docs/11-core.md](docs/11-core.md) | `doido-core` | Shared errors, inflector, utilities, Active Support analogue |
| [docs/12-cable.md](docs/12-cable.md) | `doido-cable` | WebSocket channels, pub/sub, Action Cable analogue |
| [docs/15-storage.md](docs/15-storage.md) | `doido-storage` | Attached-file storage: blobs, polymorphic attachments, pluggable services (disk/memory/S3/R2/Azure/GCS) + custom-adapter registry, Active Storage analogue |
| [docs/16-auth.md](docs/16-auth.md) | `doido-auth` | Unified authentication: generic `AuthUser`, axum extractors, cookie/JWT/OAuth strategies, optional 2FA, pre-built session/registration routes, Devise analogue |

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
├── doido-storage/      ← attached-file storage (blobs, attachments, disk/memory/S3/R2/Azure)
├── doido-cable/        ← websocket channels + pub/sub
└── doido-auth/         ← unified authentication (extractors, cookie/JWT/OAuth, optional 2FA)
```

## Interview Status

- [x] 01-router — **Macro DSL, `resources!` with all 7 REST routes, `only:`/`except:`, namespace/scope; axum via `doido_controller::axum`. API mode (`[app] api_only` in `config/application.toml`) drops `new`/`edit` form routes at compile time.**
- [x] 02-controller — **`#[controller]` macro + trait, `#[before_action]`/`#[after_action]` attrs, controller helpers (`#[helper]`, `app/helpers/`), Tower middleware layers; re-exports axum**
- [x] 03-model — **Re-exports sea-orm + sea-orm-migration + sea-orm-cli (`cli` feature); connection pool + test helpers (SQLite in-memory). Import only via `doido_model::sea_orm*` — never direct upstream deps**
- [x] 04-view — **Tera default engine, swappable via `TemplateEngine` trait, convention-based template resolution**
- [x] 05-config — **Per-env YAML (`config/<env>.yml`), AES-256-GCM encrypted credentials (`credentials.yml.enc` + `master.key`), `SECTION__KEY` env override; layered TOML dropped (US-085)**
- [x] 06-cli — **Runtime commands only; `doido generate` delegates to `doido-generators`**
- [x] 06b-generators — **Separate crate, all Rails targets, `Generator` trait registry, `generate helper`, scaffold/controller emit helpers, auto-injects `config/routes.rs`**
- [x] 07-middleware — **Logging+PanicRecovery always-on, all else opt-in via config, pluggable `SessionStore` (cookie default). API mode (`MiddlewareStack::with_api_only`, read from `[app] api_only`) skips HTML-only middleware like CSRF.**
- [x] 08-mailer — **`deliver_now()` + `deliver_later()`, templates in `views/mailers/`, pluggable `Deliverer` trait**
- [x] 09-jobs — **Pluggable backends (memory/db/redis), exponential retry per-job via `#[job]` macro, dead letter queue + CLI**
- [x] 10-cache — **Pluggable backends (memory/redis/db), configurable namespacing (`app:env:custom:key`), multiple named stores**
- [x] 11-core — **`thiserror` per crate + `anyhow` at app level, all inflections + `config/inflections.rs` for custom rules**
- [x] 12-cable — **`#[channel]` macro + trait, pluggable PubSub (memory/redis/db), middleware+`CableConnection` auth, ActionCable wire protocol, generator added**
- [x] 15-storage — **Pluggable `Service` (disk default/memory/S3/R2/Azure/GCS + custom-adapter registry via `register_adapter`/`type:`), blobs+polymorphic attachments (raw SQL), HMAC signed ids/URLs, axum redirect+proxy+direct-upload serving, `storage:install`/`storage:adapter` generators; variants/previews deferred**
- [x] 16-auth — **`AuthUser` trait, axum extractors (`CurrentUser`/`MaybeUser`/`RequireAuth`/`AuthToken`), pluggable strategies (cookie/JWT/OAuth2), optional 2FA (`auth-2fa`), pre-built `routes::mount`, generators in `doido-auth` (CLI-visible when installed); `doido new --auth` bootstrap. Release e2e `auth_install` scaffolded (`#[ignore]`).**

## Tutorial & docs standard (MUST follow)

Tutorials under `docs-blog/content/docs/tutorials/` are executable specs. When writing or
editing one:

1. **Create controllers with generators**, never a hand-written `#[controller]` skeleton the
   reader types from scratch — use `cargo doido generate scaffold <Name> …` for a resource or
   `cargo doido generate controller <Name>` for a one-off; only *customize* the generated files.
   (`scaffold` regenerates the model, so don't also `generate model` the same name.)
2. **Routes come after the controller they reference.** Generators auto-inject the route with the
   controller; any manual route edit is shown only after that controller exists — a reader
   following top-to-bottom never points a route at a controller that isn't there yet.
3. **Mirror every tutorial with an e2e scenario** in `doido-generators/tests/e2e/scenarios/` that
   runs the same generator script + customizations, builds under `-D warnings`, and asserts the
   behavior over HTTP. Tutorial code blocks and the scenario's embedded code are one source of
   truth — change one, change the other. Pair: `building-a-blog.md` ↔ `blog_tutorial.rs`
   (`make release-e2e`). Full rationale: [docs/06b-generators.md](docs/06b-generators.md#tutorial-standard-docs--e2e).
