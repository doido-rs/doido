# Doido — Implementation-State Architecture (authoritative)

`CLAUDE.md` indexes the **spec** documents (the design intent). This file is the
authoritative map of what is **actually built** in the workspace, the crate ↔ spec
status, and the reconciliation decisions where the specs and the code diverge. It
is the source of truth for the implementation backlog (`prd.json`).

Last reconciled: 2026-07-28 (branch `cleaning_the_specs`), verified spec-by-spec
against the crate source. The prior reconciliation (2026-07-24, `first_stable_project`)
predates the 104-story backlog (`harness/prd.json`, US-001→US-104) that closed most of
the Rails-8 gap list — several statuses below were upgraded accordingly.

## Actual workspace (from `Cargo.toml`)

15 members: 10 library crates + 4 proc-macro crates + the `doido` meta crate.
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
| `doido-controller` | 01, 02, 07 | Done | `routes!` + `#[controller]` + filters (`before/after/around_action`, `skip_before_action`, `only/except`) + Tower stack (logging + panic always-on; CORS/CSRF/force-SSL/host-allowlist/rate-limit opt-in) + strong params (`permit/require`) + `respond_to`/format negotiation + cookies + health check + per-env YAML config + `SECTION__KEY` env overrides. **Gaps:** session is not exposed on `Context` as `ctx.session` (only via middleware `SessionStore`); cookie session is HMAC-signed but not AES-256-GCM encrypted. |
| `doido-controller/macros` | 01, 02 | Done | `routes!`, `#[controller]`; `before/after/around_action` codegen works. |
| `doido-model` | 03 | Done | sea-orm re-export + connection pool + Rails-style schema builders + db tasks (`seeds`, `tasks` reset/setup/prepare, `schema` dump/load, `migrate` rollback/redo, pool knobs) + `TestDb` (incl. `TestDb::run_migrations::<M>()` / `TestDb::seed()` convenience helpers). No open gaps. |
| `doido-view` | 04 | Done | Tera engine (swappable) + `ViewResponse` + layouts/partials + helpers (`asset`, `form`, `link`, `tag`, `sanitize`, `i18n`, `number`, `hotwire`). |
| `doido-cache` | 10 | Done | memory + redis + memcache + db backends + named registry (`store()`) + `fetch` + `read/write/fetch_multi` + namespacing + compression. No open gaps. |
| `doido-storage` | 15 | Done | pluggable `Service` (disk/memory/S3/R2/Azure/GCS + custom-adapter registry) + blobs/attachments (raw SQL) + HMAC signed URLs + axum serving (redirect/proxy/direct-upload) + `storage:install`/`storage:adapter` generators. **Deferred (per spec):** variants, previews, video/audio analyzers, Mirror, native Azure SAS, `#[has_one_attached]`, `compose`. |
| `doido-jobs` | 09 | Done | `JobQueue` trait + worker engine + backoff + memory/db/redis backends + leasing + dead-letter. **Gap:** the spec's fluent per-instance enqueue (`Job{}.enqueue()/.enqueue_at()/.on_queue()`) is absent — enqueue is via `queue.enqueue(payload)` + the macro-generated `*_enqueue()` fn. |
| `doido-jobs/macros` | 09 | Done | `#[job]` + generated `*_enqueue()` helper. |
| `doido-mailer` | 08 | **Partial** | `Mail` + `Deliverer` + `Log`/`Test`/**`Smtp`**/**`Sendmail`** deliverers + MIME assembly + `deliver_now`/`deliver_later` (via `doido-jobs`) + previews + **real `#[mailer]` macro** all work. **Gap:** mailers do **not** render `views/mailers/<m>/<action>.{html,text}.tera` via `doido-view` (no view dependency) — bodies are set manually; no HTML→text fallback, no `Mail::template(...)`, no multipart-from-templates; `Mail.to` is single (no cc/bcc); SMTP has no TLS/STARTTLS. |
| `doido-mailer/macros` | 08 | Done | real codegen (implements the mailer trait + template-key resolution). |
| `doido-cable` | 12 | **Partial** | ActionCable wire frames + `PubSub` (memory/redis/db) + `Channel` trait + **real `#[channel]` macro** + heartbeat helpers work. **Gap:** there is **no WebSocket server** — no `axum::extract::ws` dependency anywhere in the workspace, no `server.rs`, no connection lifecycle, no `ChannelRegistry`, no subscription routing/dispatch, no `ctx.stream_from/transmit/params/stop_all_streams`, no `cable!()` route macro. It is a protocol + pub/sub library, not a live server. |
| `doido-cable/macros` | 12 | Done | real codegen (implements the channel trait + name resolution). |
| `doido-generators` | 06, 06b | Done | CLI (`new`/`generate`/`server`/`console` via evcxr/`db`/`worker`) + generator registry + generators (model, controller, migration, scaffold, resource, mailer, job, channel, storage_install/adapter, templates, generator, locale) + route auto-injection + embedded templates. `doido db` wires `create`/`reset`/`prepare`/`seed`/`schema dump|load` (delegating to `doido-model` tasks; `db/schema.sql`/`db/seeds.sql` conventions) plus the SeaORM passthrough; `jobs:failed/retry/discard` are backed by the dead-letter store (`discard_dead` trait method + per-backend impls). **Gaps:** `credentials:edit` is a log-only stub and `credentials:show` is absent; `server` does not parse `--port`/`--env`; the jobs CLI still uses the default (memory) backend until `[jobs]` config is wired (same TODO as the worker). |
| `doido` (meta) | all | Done | re-exports + `run()` entry. |
| `doido-config` | 05 | **Partial / decided** | Reality is per-env **YAML** (`config/<env>.yml`) via `YamlConfig` (split across `doido-controller` + `doido-model`) + `SECTION__KEY` env overrides (`doido_controller::env_override`) + an initializers boot registry. **Deferred (backlog):** the spec's layered **TOML** (base→env→credentials→env), **AES-256-GCM** encrypted credentials + `config/master.key`, and the `doido credentials edit/show` CLI. |

## Reconciliation decisions

These are the working defaults. Flagged items are genuine product decisions — override
here and the backlog follows.

1. **Config — YAML now, encrypted credentials deferred.** Reality is per-env
   `config/<env>.yml` (a `Config` trait + `YamlConfig` folded into `doido-controller`
   and `doido-model`). Spec 05 asks for layered **TOML** + **AES-256-GCM credentials** +
   `SECTION__KEY` env overrides. **Decision (US-085):** standardize on per-env **YAML**,
   the implemented and tested path. `SECTION__KEY` env overrides exist
   (`doido_controller::env_override`); an initializers registry exists. The template's
   `config/application.toml` is a minimal placeholder only. Layered TOML + AES-256-GCM
   credentials + the credentials CLI stay **deferred (opt-in, vNext)**. Spec 05 has been
   annotated to reflect this.

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
7. **Jobs worker** (separate process) — `doido worker` drives the `WorkerEngine`.
8. **HTTP server** — `doido-controller` mounts the `routes!` table on axum and listens.

> There is **no committed `examples/blog` app**. The end-to-end definition-of-done is the
> `make example` target (US-104): it scaffolds an ephemeral `blog` (`--api`) under
> `target/`, runs `db create`/`db migrate` on a temp SQLite DB, boots the app's own
> server, and exercises POST/GET/PATCH/DELETE `/posts`. (`docs-blog/` is an unrelated
> markdown content site, not a demo app.)

## Backlog seeds (open spec gaps — feed `prd.json`)

Ordered by size. These are the real spec-vs-code gaps after the 104-story backlog:

1. **`doido-cable` — WebSocket server.** axum `ws` upgrade handler, connection lifecycle
   + ping loop, `ChannelRegistry`, subscription routing/dispatch, `ctx.stream_from/
   transmit/params/stop_all_streams`, and the `cable!()` route macro (spec 12).
2. **`doido-mailer` — template rendering.** Wire `doido-view`: render
   `views/mailers/<m>/<action>.{html,text}.tera`, HTML→text fallback, multipart assembly,
   `Mail::template(...)`, cc/bcc, SMTP TLS/STARTTLS (spec 08).
3. **Config — encrypted credentials + layered TOML.** AES-256-GCM `credentials.toml.enc`,
   `config/master.key`/`DOIDO_MASTER_KEY`, `doido credentials edit/show`, and (if adopted)
   the base→env→credentials→env-var TOML layering (spec 05) — *if adopted*.
4. **Small wiring / ergonomics.**
   - ~~Wire `doido db seed/reset/prepare/schema` into the CLI~~ **(done, Phase 1).**
   - ~~Real `doido jobs:failed/retry/discard` backed by the dead-letter queue~~ **(done, Phase 1).**
   - ~~`testing::run_migrations()`/`seed()` helpers (spec 03)~~ **(done, Phase 1).**
   - ~~`routes!` `HEAD`/`OPTIONS` verbs~~ **(done, Phase 1)**; route constraints DSL still open (spec 01).
   - Expose `ctx.session` (and flash) on `Context` (spec 02); AES-256-GCM cookie encryption (spec 07).
   - Fluent per-instance `Job{}.enqueue()/.enqueue_at()/.on_queue()` (spec 09).
