# Doido — Implementation-State Architecture (authoritative)

`CLAUDE.md` indexes the **spec** documents (the design intent). This file is the
authoritative map of what is **actually built** in the workspace, the crate ↔ spec
status, and the reconciliation decisions where the specs and the code diverge. It
is the source of truth for the implementation backlog (`prd.json`).

Last reconciled: 2026-07-24 (branch `first_stable_project`).

## Actual workspace (from `Cargo.toml`)

14 members: 9 library crates + 4 proc-macro crates + the `doido` meta crate.
Several specced crates were **merged**, so they do not exist as separate crates:

- `doido-router` → merged into **`doido-controller`** (`routes!` macro lives there).
- `doido-middleware` → merged into **`doido-controller`** (`MiddlewareStack`, sessions).
- `doido-cli` → merged into **`doido-generators`** (`new`/`generate`/`server`/`db`/`worker`).

## Crate ↔ spec status

Legend: **Done** = implemented + tested · **Partial** = core works, spec features missing ·
**Stub** = placeholder only · **Deferred** = not in first stable (exists only in the
`.worktrees/generators-new-generate` experimental worktree).

| Crate | Spec | Status | Notes |
|-------|------|--------|-------|
| `doido-core` | 11 | Done | errors, logger, inflector (+ custom rules) |
| `doido-controller` | 01, 02, 07 | Done | `routes!` + `#[controller]` + filters + Tower middleware + cookie session + per-env YAML config |
| `doido-controller/macros` | 01, 02 | Done | `routes!`, `#[controller]`; `before/after_action` codegen works (the standalone attr macros are inert shells) |
| `doido-model` | 03 | Done | sea-orm re-export + connection pool + Rails-style schema builders + `testing` helpers |
| `doido-view` | 04 | Done | Tera engine (swappable) + `ViewResponse` + global registry |
| `doido-cache` | 10 | Done | memory + redis + memcache + named registry + namespacing |
| `doido-jobs` | 09 | Done | queue + worker + backoff + memory/db/redis backends + dead-letter |
| `doido-jobs/macros` | 09 | Done | `#[job]` + generated `*_enqueue()` helper |
| `doido-mailer` | 08 | **Partial** | `Mail` + `Deliverer` + `Log`/`Test` deliverers work; **`#[mailer]` macro is a stub** |
| `doido-mailer/macros` | 08 | **Stub** | pass-through, no codegen |
| `doido-cable` | 12 | **Partial** | `Cable` + `MemoryPubSub` broadcast work; **`Channel` trait + ActionCable protocol are minimal**; `#[channel]` stub |
| `doido-cable/macros` | 12 | **Stub** | pass-through, no codegen |
| `doido-generators` | 06, 06b | Done | CLI (`new`/`generate`/`server`/`db`/`worker`) + generator registry + embedded templates; `console` is a placeholder |
| `doido` (meta) | all | Done | re-exports + `run()` entry |
| `doido-config` | 05 | **Deferred / Partial** | basic per-env YAML lives folded in `controller`/`model`; the spec's TOML-layering, AES-256-GCM credentials and env overrides are **not implemented**; a WIP crate exists in the worktree |
| `doido-kafka` | 13 | **Deferred** | ~52-line skeleton in the worktree only |
| `doido-mcp` | 14 | **Deferred** | ~216-line WIP in the worktree only |

## Reconciliation decisions

These are the working defaults for `first_stable_project`. Flagged items are genuine
product decisions — override here and the backlog follows.

1. **Config — YAML now, encrypted credentials deferred.** Reality is per-env
   `config/<env>.yml` (a `Config` trait + `YamlConfig` in `doido-controller`), plus
   `config/application.toml` in the app template. Spec 05 asks for layered **TOML** +
   **AES-256-GCM credentials** + `SECTION__KEY` env overrides — none of that exists yet.
   *Decision (default):* ship the current YAML config for first stable; extract a
   dedicated `doido-config` crate later (seeding from the worktree WIP) and track
   layering + credentials + env overrides as backlog items.
   **Drift resolved (2026-07-25):** standardize on per-env **YAML**
   (`config/<env>.yml`), the implemented and tested path. `SECTION__KEY` env
   overrides now exist (`doido_controller::env_override`). The template's
   `config/application.toml` is a minimal placeholder only; layered TOML +
   AES-256-GCM credentials stay deferred (opt-in, vNext) and can be revisited if
   an app needs them.
2. **Kafka — Deferred (opt-in, vNext).** Not part of first stable; promote the worktree
   crate when scheduled.
3. **MCP — Deferred (opt-in, vNext).** Same as kafka.

## Runtime boot sequence (closes the "how does it wire together" gap)

A running Doido app initializes process-global singletons at boot, in dependency order,
then serves. Concrete wiring lives in the `doido-generators` `server` command; the
generated app's `src/main.rs` calls `doido_generators::run(Some(routes))`.

1. **Logger** — `doido_core` tracing subscriber.
2. **Config** — load per-env YAML (`doido_controller::YamlConfig` for the current `Environment`).
3. **DB pool** — `doido_model::pool::init()` → `&'static DatabaseConnection`.
4. **View engine** — `doido_view::init("app/views")`.
5. **Cache** — `doido_cache::global::init()` → `Arc<dyn CacheStore>`.
6. **Jobs worker** (separate process) — `doido worker` drives the `WorkerEngine`.
7. **HTTP server** — `doido-controller` mounts the `routes!` table on axum and listens.

> The `examples/blog` app (definition-of-done) makes this sequence executable and is
> the reference for the exact wiring.

## Backlog seeds (feed `prd.json` in Fase 5)

- Implement `#[mailer]` macro expansion per spec 08 (+ TDD surface).
- Implement `#[channel]` macro + ActionCable wire protocol + `Channel` trait per spec 12 (+ TDD).
- Flesh out `doido-cable` protocol / subscribe / broadcast beyond `MemoryPubSub`.
- Resolve `before/after_action` inert attr shells (remove or make meaningful).
- `doido-generators` interactive `console` REPL (currently a placeholder).
- Config: encrypted credentials + TOML layering + env overrides (spec 05) — *if adopted*.
- `examples/blog` end-to-end app as the framework definition-of-done.
- Promote `doido-kafka` / `doido-mcp` from the worktree when scheduled.
