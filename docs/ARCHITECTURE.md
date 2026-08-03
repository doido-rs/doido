# Doido — Implementation-State Architecture (authoritative)

`CLAUDE.md` indexes the **spec** documents (the design intent). This file is the
authoritative map of what is **actually built** in the workspace, the crate ↔ spec
status, and the reconciliation decisions where the specs and the code diverge. It
is the source of truth for the implementation backlog (`prd.json`).

Last reconciled: 2026-08-03 (branch `creating_doido_auth`), added spec 16 (`doido-auth`)
and harness stories US-105→US-113. Prior reconciliation: 2026-07-28 (`cleaning_the_specs`).

## Actual workspace (from `Cargo.toml`)

15 members today: 10 library crates + 4 proc-macro crates + the `doido` meta crate.
**Planned:** `doido-auth` (+ optional `doido-auth/macros`) — see spec 16 and
`harness/prd.json` US-105→US-113.
Several specced crates were **merged**, so they do not exist as separate crates:

- `doido-router` → merged into **`doido-controller`** (`routes!` macro lives there).
- `doido-middleware` → merged into **`doido-controller`** (`MiddlewareStack`, sessions).
- `doido-cli` → merged into **`doido-generators`** (`new`/`generate`/`server`/`db`/`worker`).
- `doido-config` → **not a crate**; config lives folded into `doido-controller` and
  `doido-model` (per-env YAML) — see the reconciliation decision below.

## Crate ↔ spec status

Legend: **Done** = implemented + tested · **Partial** = core works, spec features missing ·
**Stub** = placeholder only · **Deferred** = deliberately out of first stable.

| Crate | Spec | Status | Notes |
|-------|------|--------|-------|
| `doido-core` | 11 | Done | errors, logger, inflector, notifications bus, `core_ext` (blank/present, String/Array/Hash), time helpers, concerns, `test_time`. Custom inflection rules load from **`config/inflection.yaml`** (runtime), not the spec's compiled `config/inflections.rs`. |
| `doido-controller` | 01, 02, 07 | Done | `routes!` + `#[controller]` + filters (`before/after/around_action`, `skip_before_action`, `only/except`) + Tower stack (logging + panic always-on; CORS/CSRF/force-SSL/host-allowlist/rate-limit opt-in) + strong params (`permit/require`) + `respond_to`/format negotiation + `ctx.session()`/`ctx.flash()`/`ctx.cookies()` (flushed to `Set-Cookie` by the `#[controller]` macro; session cookie **AES-256-GCM** encrypted via `doido_core::crypto`, secret from `doido_controller::secret`) + `routes!` `constraints: { id: numeric }` DSL + health check + per-env YAML config + `SECTION__KEY` env overrides. No open gaps. |
| `doido-controller/macros` | 01, 02 | Done | `routes!`, `#[controller]`; `before/after/around_action` codegen works. |
| `doido-model` | 03 | Done | sea-orm re-export + connection pool + Rails-style schema builders + db tasks (`seeds`, `tasks` reset/setup/prepare, `schema` dump/load, `migrate` rollback/redo, pool knobs) + `TestDb` (incl. `TestDb::run_migrations::<M>()` / `TestDb::seed()` convenience helpers). No open gaps. |
| `doido-view` | 04 | Done | Tera engine (swappable) + `ViewResponse` + layouts/partials + helpers (`asset`, `form`, `link`, `tag`, `sanitize`, `i18n`, `number`, `hotwire`). |
| `doido-cache` | 10 | Done | memory + redis + memcache + db backends + named registry (`store()`) + `fetch` + `read/write/fetch_multi` + namespacing + compression. No open gaps. |
| `doido-storage` | 15 | Done | pluggable `Service` (disk/memory/S3/R2/Azure/GCS + custom-adapter registry) + blobs/attachments (raw SQL) + HMAC signed URLs + axum serving (redirect/proxy/direct-upload) + `storage:install`/`storage:adapter` generators. **Deferred (per spec):** variants, previews, video/audio analyzers, Mirror, native Azure SAS, `#[has_one_attached]`, `compose`. |
| `doido-jobs` | 09 | Done | `JobQueue` trait (incl. `discard_dead`) + worker engine + backoff + memory/db/redis backends + leasing + dead-letter + config loading (`config::load`/`build_configured_queue` reading the `jobs:` section of `config/<env>.yml`, mirroring `doido-cache`). Enqueue via `queue.enqueue(payload)`, the macro-generated `*_enqueue()` fn, or the fluent per-instance builder `<Name>Job::new(payload).on_queue(q).wait(secs).enqueue(&q)` (`#[job]`-generated). No open gaps. |
| `doido-jobs/macros` | 09 | Done | `#[job]` + generated `*_enqueue()` helper. |
| `doido-mailer` | 08 | Done | `Mail` (to + cc/bcc, `recipients()`) + `Deliverer` + `Log`/`Test`/`Smtp`/`Sendmail` deliverers + MIME assembly (Cc header; Bcc envelope-only) + `deliver_now`/`deliver_later` (via `doido-jobs`) + previews + `#[mailer]` macro. `Mailer::mail(action, ctx)` renders `mailers/<m>/<action>.{html,text}.tera` via `doido-view` with an HTML→text fallback. SMTP does multi-recipient `RCPT TO` and opt-in `STARTTLS` (`SmtpDeliverer::new(addr).starttls()`, rustls). No open gaps. |
| `doido-mailer/macros` | 08 | Done | real codegen (implements the mailer trait + template-key resolution). |
| `doido-cable` | 12 | Done | ActionCable wire frames + `PubSub` (memory/redis/db) + `Channel` trait + `#[channel]` macro + heartbeat helpers + **live WebSocket server** (`server.rs`): axum `ws` upgrade handler, connection lifecycle + heartbeat ping loop, `ChannelRegistry`, subscription routing/dispatch (subscribe→confirm/reject, message, unsubscribe), `ctx.transmit/stream_from/params/stop_all_streams`, pub/sub bridge, and the `cable!(pubsub, [Channels…])` route macro. E2E-tested over a real WebSocket. No open gaps. |
| `doido-cable/macros` | 12 | Done | real codegen (implements the channel trait + name resolution). |
| `doido-generators` | 06, 06b | Done | CLI (`new`/`generate`/`server`/`console` via evcxr/`db`/`worker`) + generator registry + generators (model, controller, migration, scaffold, resource, mailer, job, channel, storage_install/adapter, templates, generator, locale) + route auto-injection + embedded templates. `doido db` wires `create`/`reset`/`prepare`/`seed`/`schema dump|load` (delegating to `doido-model` tasks; `db/schema.sql`/`db/seeds.sql` conventions) plus the SeaORM passthrough; `jobs:failed/retry/discard` are backed by the dead-letter store (`discard_dead` trait method + per-backend impls). `credentials edit`/`show` encrypt/decrypt `config/credentials.yml.enc` with AES-256-GCM (`doido_core::crypto`), keyed by `config/master.key` (auto-generated + gitignored) or `DOIDO_MASTER_KEY`. `doido server --port/--env` override the bind port and environment; `doido worker`/`doido jobs` pick the backend (memory/db/redis), queues, and concurrency from the `jobs:` section of `config/<env>.yml` (`doido_jobs::config::load` + `build_configured_queue`; db/redis backends compiled into the CLI). **Gap:** the worker's job dispatch is still a stub (logs + acks) — a job-type registry mapping each `#[job]` to its `perform()` is not yet wired. |
| `doido` (meta) | all | Done | re-exports + `run()` entry. |
| `doido-config` | 05 | **Partial / decided** | Reality is per-env **YAML** (`config/<env>.yml`) via `YamlConfig` (split across `doido-controller` + `doido-model`) + `SECTION__KEY` env overrides (`doido_controller::env_override`) + an initializers boot registry. **AES-256-GCM encrypted credentials** (`config/credentials.yml.enc` + `config/master.key`/`DOIDO_MASTER_KEY`) with the `doido credentials edit/show` CLI are implemented (Phase 5, `doido-generators` + `doido_core::crypto`). **Still deferred:** auto-injecting decrypted credentials into the config tree. (Layered TOML was dropped from spec 05 — YAML is the decided path.) |
| `doido-auth` | 16 | **Not started** | Spec only (2026-08-03). Target: generic `AuthUser` trait, axum extractors, pluggable strategies (cookie/JWT/OAuth+OAuth2), optional 2FA (feature `auth-2fa`), pre-built `auth_routes!`, **generators owned by this crate** (`auth:install`/`auth:controller`/`auth:scaffold` via `doido_auth::generators::register`) visible in CLI only when `doido-auth` is a project dependency. Builds on `doido-model::password` + `doido-controller` sessions. Backlog: US-105→US-113. |

## Reconciliation decisions

These are the working defaults. Flagged items are genuine product decisions — override
here and the backlog follows.

## Dependency import conventions

Third-party crates that Doido wraps are **never imported directly** from workspace
crates (except inside the crate that owns the re-export). Application code generated
by `doido new` uses the **`doido` meta crate**; first-party workspace code uses the
specific crate path.

| Upstream | Import through | Notes |
|----------|----------------|-------|
| `sea_orm` | `doido_model::sea_orm` | Enable `sqlite` / `postgres` / `mysql` on `doido-model` (or the meta `doido` feature) for the SQL driver |
| `sea_orm_migration` | `doido_model::sea_orm_migration` | Migration crates depend on `doido-model` only — no direct `sea-orm-migration` dep |
| `sea_orm_cli` | `doido_model::sea_orm_cli` | Feature `cli` on `doido-model`; used by `doido db` |
| `axum` | `doido_controller::axum` | `doido-controller` re-exports axum (incl. `ws` for cable/storage serving) |
| Auth extractors / strategies | `doido_auth::…` | Planned crate (spec 16); JWT/OAuth upstream deps re-exported from `doido-auth` only |
| Other workspace crates | `doido_<crate>` directly | e.g. `doido_storage`, `doido_jobs` — **not** `doido::storage` inside this repo |
| Meta crate `doido` | `doido::…` | **Generated apps and external consumers only** |

Inside `doido-model` / `doido-controller`, use `crate::sea_orm`, `crate::sea_orm_migration`,
and `crate::axum` respectively (the re-export layer).

1. **Config — per-env YAML.** Reality is per-env `config/<env>.yml` (a `Config` trait +
   `YamlConfig` folded into `doido-controller` and `doido-model`). **Decision (US-085):**
   standardize on per-env **YAML**, the implemented and tested path; a base-then-env
   layered format (e.g. TOML) was dropped and is **no longer a spec item** (spec 05 rewritten
   around YAML). `SECTION__KEY` env overrides exist (`doido_controller::env_override`); an
   initializers registry exists. **AES-256-GCM encrypted credentials** + the `doido
   credentials edit/show` CLI are implemented (Phase 5). The only remaining follow-up is
   auto-injecting decrypted credentials into the config tree.

2. **Inflection rules — YAML, not a compiled Rust file.** Spec 11 describes
   `config/inflections.rs` (a compiled `configure(&mut Inflections)`); the implementation
   loads rules at runtime from `config/inflection.yaml`. Functionally equivalent; spec 11
   annotated. (Revisit only if compile-time inflection config is wanted.)

## Runtime boot sequence (closes the "how does it wire together" gap)

A running Doido app initializes process-global singletons at boot, in dependency order,
then serves. Concrete wiring lives in the `doido-generators` `server` command; the
generated app's `src/main.rs` calls `doido_generators::run(Some(routes))`.

1. **Logger** — `doido_core` tracing subscriber.
2. **Config** — load per-env YAML (`doido_controller::YamlConfig` for the current `Environment`), apply `SECTION__KEY` overrides, run initializers.
3. **DB pool** — `doido_model::pool::init()` → `&'static DatabaseConnection`.
4. **View engine** — `doido_view::init("app/views")`.
5. **Cache** — `doido_cache::global::init()` → `Arc<dyn CacheStore>`.
6. **Storage** — `doido_storage::Storage::from_config(db)` builds the configured
   `Arc<dyn Service>` + signer; `serving::routes()` mounts blob/direct-upload endpoints.
7. **Storage** — `doido_storage::Storage::from_config(db)` builds the configured
   `Arc<dyn Service>` + signer; `serving::routes()` mounts blob/direct-upload endpoints.
8. **Auth** (when installed) — `doido_auth::init(db, &config.auth)` registers strategies
   and OAuth providers; `doido_auth::layer()` wraps the router.
9. **Jobs worker** (separate process) — `doido worker` drives the `WorkerEngine`.
10. **HTTP server** — `doido-controller` mounts the `routes!` table on axum and listens.

> There is **no committed `examples/blog` app**. The end-to-end definition-of-done is the
> `make example` target (US-104): it scaffolds an ephemeral `blog` (`--api`) under
> `target/`, runs `db create`/`db migrate` on a temp SQLite DB, boots the app's own
> server, and exercises POST/GET/PATCH/DELETE `/posts`. (`docs-blog/` is an unrelated
> markdown content site, not a demo app.)

## Backlog seeds (open spec gaps — feed `prd.json`)

Ordered by size. These are the real spec-vs-code gaps after the 104-story backlog:

1. ~~**`doido-cable` — WebSocket server.**~~ **(done, Phase 4)** — axum `ws` upgrade handler,
   connection lifecycle + ping loop, `ChannelRegistry`, subscription routing/dispatch,
   `ctx.stream_from/transmit/params/stop_all_streams`, and the `cable!()` route macro (spec 12).
2. ~~**`doido-mailer` — template rendering.**~~ **(done, Phase 3)** — `Mailer::mail()`
   renders `mailers/<m>/<action>.{html,text}.tera` via `doido-view` with HTML→text fallback;
   cc/bcc + multi-recipient `RCPT TO`; opt-in SMTP `STARTTLS` (rustls) (spec 08).
3. ~~**Config — encrypted credentials.**~~ **(done, Phase 5)** — AES-256-GCM
   `config/credentials.yml.enc`, `config/master.key`/`DOIDO_MASTER_KEY`, `doido credentials
   edit/show`. *Still deferred:* auto-injecting decrypted credentials into the config tree.
   (Layered TOML was removed from spec 05 — YAML is the decided path.)
4. **Small wiring / ergonomics.**
   - ~~Wire `doido db seed/reset/prepare/schema` into the CLI~~ **(done, Phase 1).**
   - ~~Real `doido jobs:failed/retry/discard` backed by the dead-letter queue~~ **(done, Phase 1).**
   - ~~`testing::run_migrations()`/`seed()` helpers (spec 03)~~ **(done, Phase 1).**
   - ~~`routes!` `HEAD`/`OPTIONS` verbs~~ **(done, Phase 1)**; ~~route constraints DSL (spec 01)~~ **(done, Phase 2)**.
   - ~~Expose `ctx.session` (and flash) on `Context` (spec 02); AES-256-GCM cookie encryption (spec 07)~~ **(done, Phase 2)**.
   - ~~Fluent per-instance `Job{}.enqueue()/.enqueue_at()/.on_queue()` (spec 09)~~ **(done, Phase 2)**.
5. **`doido-auth` — unified authentication (spec 16).** New crate + generators; see
   `harness/prd.json` US-105→US-113. Password hashing already exists in
   `doido-model::password`; auth adds extractors, strategies, routes, and install/scaffold
   generators **inside `doido-auth`**. The CLI merges them via `Cargo.toml` detection
   (`doido-generators` US-112). **Deferred within auth:** `#[auth_user]` proc-macro,
   SAML/WebAuthn, magic-link.
