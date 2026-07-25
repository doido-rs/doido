# Doido ← Rails 8 — Feature Gap Analysis

This document lists the features **Rails 8 has that Doido does not yet fully have**,
grouped by module. It is a decision tool for the implementation backlog.

## How to use this file

- Every gap is **one line, one checkbox**. Skim, then:
  - **Delete the whole line** for anything you will *not* implement.
  - Flip `- [ ]` → `- [x]` when a feature ships.
- Sections mirror the crate / spec layout, so you can prune a whole module at once.
- Only **gaps** are listed here (missing or partial). Everything already done lives in
  [ARCHITECTURE.md](ARCHITECTURE.md) — this file is intentionally the "what's left" view.

### Legend

Each line reads: `- [ ] `[tag]` **Feature** — note. *(state)*`

**Priority tags**
- `[core]` — capability most Rails apps depend on (auth, validations, CSRF, forms…).
- `[nice]` — ergonomic/convenience feature; an app works without it.
- `[deferred]` — already explicitly deferred in ARCHITECTURE.md (Kafka, MCP, encrypted
  credentials / TOML config layering).

**State markers**
- `*(missing)*` — no implementation exists.
- `*(partial)*` — some code exists but it's incomplete or a stub; the note says what's there.

> Derived from the 2026-07-24 reconciliation (branch `first_stable_project`). The code
> moves fast — spot-check a line against the source before you build it.

---

## Router / Action Dispatch — `doido-controller` (macros)

- [ ] `[core]` **Root route** (`root "home#index"`) — *(missing)*
- [x] `[core]` **`respond_to` / format-based content negotiation** — *(missing)*
- [ ] `[core]` **Named path/URL helpers for custom routes** — only `resources!` generates `_path` helpers; plain `get!/post!` routes get none. *(partial)*
- [ ] `[core]` **`member` / `collection` route blocks on resources** — must add manual method routes today. *(partial)*
- [ ] `[nice]` **Route constraints** (regex / format / lambda) — *(missing)*
- [ ] `[nice]` **Singular resource** (`resource :profile`) — *(missing)*
- [ ] `[nice]` **Shallow nesting** for nested resources — *(missing)*
- [ ] `[nice]` **Redirect routes** (`to: redirect("/x")`) — *(missing)*
- [ ] `[nice]` **Glob / catch-all routes** (`/*path`) — *(missing)*
- [ ] `[nice]` **`mount` engines / sub-apps** + `direct`/`resolve` custom helpers — *(missing)*

## Controller / Action Controller — `doido-controller`

- [x] `[core]` **Strong parameters** (`permit`/`require` allowlist) — `ctx.form/params` deserialize directly, no filtering. *(missing)*
- [x] `[core]` **CSRF protection** (authenticity token) — *(missing)*
- [x] `[core]` **Flash messages** — *(missing)*
- [x] `[core]` **Cookies API** (read/write, signed/encrypted) — *(missing)*
- [x] `[core]` **`rescue_from`** typed error handling — only panic→500 today. *(missing)*
- [x] `[core]` **`skip_before_action`** / filter skipping — *(missing)*
- [ ] `[core]` **Real `#[before_action]` / `#[after_action]` attrs** — standalone attrs are inert shells (the codegen path in `#[controller]` works). *(partial)*
- [x] `[nice]` **`around_action`** filters — *(missing)*
- [x] `[nice]` **HTTP caching** (`fresh_when` / `stale?` / ETag / Last-Modified) — *(missing)*
- [x] `[nice]` **Rate limiting** (Rails 8 `rate_limit`) — *(missing)*
- [x] `[nice]` **`force_ssl`** / SSL redirect — *(missing)*
- [x] `[nice]` **Response streaming** / `send_data` / `send_file` — *(missing)*

## Middleware & Sessions — `doido-controller`

- [x] `[core]` **Real cookie session store** (signed/encrypted values) — `CookieSessionStore` is a no-op stub. *(partial)*
- [x] `[core]` **Server-side session backends** (cache/db-backed) — *(missing)*
- [x] `[nice]` **Config-driven CORS** (opt-in per spec) — permissive layer exists but isn't wired from config. *(partial)*
- [x] `[nice]` **Host authorization** (`config.hosts`) — *(missing)*
- [x] `[nice]` **Configurable middleware insertion** (insert before/after) — *(missing)*

## View / Action View — `doido-view`

- [ ] `[core]` **Partial + collection render helpers** (`render "form"`, `render collection:`) — Tera `include` only, no Rails-style helper. *(partial)*
- [ ] `[core]` **`content_for` / named `yield` blocks** — only `content_for_layout` exists. *(partial)*
- [ ] `[core]` **Form builders** (`form_with`/`form_for` + field helpers) — *(missing)*
- [ ] `[core]` **Link/URL helpers** (`link_to`, `button_to`) — *(missing)*
- [ ] `[core]` **Asset helpers** (`image_tag`, `stylesheet_link_tag`, `javascript_include_tag`) — *(missing)*
- [ ] `[nice]` **Tag helpers** (`tag`, `content_tag`) — *(missing)*
- [ ] `[nice]` **Number/date/currency formatting helpers** — *(missing)*
- [ ] `[nice]` **i18n view helpers** (`t`, `l`) — *(missing)*
- [ ] `[nice]` **HTML sanitization helpers** (`sanitize`, `strip_tags`) — *(missing)*
- [ ] `[nice]` **Fragment caching** (`cache` in views) — *(missing)*

## Model / Active Record — `doido-model`

- [x] `[core]` **Validations** (presence/uniqueness/format/length/numericality/custom) — *(missing)*
- [x] `[core]` **Callbacks** (before/after save/create/update/destroy/validation) — *(missing)*
- [x] `[core]` **Declarative associations ergonomics** (has_many/belongs_to/through) — raw sea-orm relations only. *(partial)*
- [x] `[core]` **Named scopes** — *(missing)*
- [x] `[core]` **Secure password / tokens** (`has_secure_password`, `generates_token_for`) — *(missing)*
- [x] `[core]` **Fixtures / factories for tests** — only in-memory `TestDb`. *(partial)*
- [ ] `[nice]` **Attribute enums with helpers** — *(missing)*
- [ ] `[nice]` **`normalizes`** (attribute normalization) — *(missing)*
- [ ] `[nice]` **`as_json` / serialized columns** — *(missing)*
- [ ] `[nice]` **Polymorphic associations / STI** — *(missing)*
- [ ] `[nice]` **Multiple databases / read-write splitting** — single global pool today. *(missing)*
- [ ] `[nice]` **Transaction convenience wrapper** — raw sea-orm only. *(partial)*
- [ ] `[deferred]` **Attribute encryption** (`encrypts`) — *(missing)*

## Migrations & DB tooling — `doido-model` + `doido db`

- [ ] `[core]` **`db:seed`** (`db/seeds` + runner) — *(missing)*
- [ ] `[nice]` **`db:schema` dump/load** (schema.rb / structure.sql) — *(missing)*
- [ ] `[nice]` **`db:reset` / `db:prepare` / `db:setup`** — *(missing)*
- [ ] `[nice]` **`db:rollback STEP=n` / redo wrappers** — sea-orm supports it, CLI wrapper is thin. *(partial)*
- [ ] `[nice]` **Connection pool size/timeout config knobs** — *(missing)*

## Jobs / Active Job — `doido-jobs`

- [ ] `[core]` **Conditional retry/discard by error type** (`retry_on` / `discard_on`) — blanket retry + dead-letter only. *(partial)*
- [ ] `[nice]` **Job lifecycle callbacks** (before/after/around perform, on-failure) — *(missing)*
- [ ] `[nice]` **`.set(wait:/wait_until:/queue:/priority:)` fluent parity** — `enqueue_at` exists. *(partial)*
- [ ] `[nice]` **Job batches / workflows** — *(missing)*
- [ ] `[nice]` **Job status introspection** (`doido jobs` dispatch) — listing only, dispatch is TODO. *(partial)*
- [ ] `[nice]` **Richer worker app-context** (beyond db handle) — *(partial)*

## Cache / Active Support Cache — `doido-cache`

- [ ] `[core]` **Read-through `fetch(key){ compute }` on miss** — no `fetch` fn in `doido-cache/src`. *(missing)*
- [ ] `[nice]` **Cache versioning** / recyclable versioned keys — *(missing)*
- [ ] `[nice]` **`read_multi` / `write_multi` / `fetch_multi`** — *(missing)*
- [ ] `[nice]` **Named/multi stores as first-class config** — registry exists but isn't config-wired. *(partial)*
- [ ] `[nice]` **Compression / pluggable serializer options** — *(missing)*

## Mailer / Action Mailer — `doido-mailer`

- [ ] `[core]` **`#[mailer]` macro codegen** — pass-through stub. *(partial)*
- [ ] `[core]` **SMTP deliverer** — only Log/Test deliverers. *(missing)*
- [ ] `[core]` **`deliver_later`** via `doido-jobs` — *(missing)*
- [ ] `[core]` **Mailer templates** (html+text via views) + MIME multipart assembly — body fields exist, no rendering/assembly. *(partial)*
- [ ] `[core]` **Attachments / inline attachments** — *(missing)*
- [ ] `[nice]` **Mailer layouts** — *(missing)*
- [ ] `[nice]` **Sendmail deliverer** — *(missing)*
- [ ] `[nice]` **Mailer previews** — *(missing)*
- [ ] `[nice]` **Interceptors / observers + i18n** — *(missing)*

## Cable / Action Cable — `doido-cable`

- [ ] `[core]` **`#[channel]` macro codegen** — pass-through stub. *(partial)*
- [ ] `[core]` **Full ActionCable wire-protocol compliance** — client frames use `type` not `command`. *(partial)*
- [ ] `[core]` **Redis PubSub backend** (multi-process broadcast) — only MemoryPubSub. *(missing)*
- [ ] `[core]` **Connection identification / auth** (`identified_by`, current_user) — minimal. *(partial)*
- [ ] `[nice]` **`stream_from` / `stream_for` naming helpers** — *(partial)*
- [ ] `[nice]` **Periodic timers / heartbeat pings** — *(missing)*
- [ ] `[nice]` **Server-side `broadcast_to` from anywhere** — *(partial)*
- [ ] `[nice]` **DB PubSub backend** — *(missing)*

## Core / Active Support — `doido-core`

- [ ] `[nice]` **`ActiveSupport::Notifications` instrumentation bus** — 4 fixed trace helpers only. *(partial)*
- [ ] `[nice]` **Core extensions** (`blank?`/`present?`, String/Array/Hash helpers) — *(missing)*
- [ ] `[nice]` **Time/Date helpers** (`2.days.ago`, `beginning_of_day`) — *(missing)*
- [ ] `[nice]` **Concerns/mixins pattern** — *(missing)*
- [ ] `[nice]` **Test time helpers** (`travel_to`, durations) — *(missing)*

## Config & Credentials — `doido-config` (deferred crate)

- [ ] `[deferred]` **Encrypted credentials** (AES-256-GCM) + `doido credentials edit` — CLI + worktree crate are stubs. *(partial)*
- [ ] `[deferred]` **Layered TOML config** (base→env→credentials→env vars) per spec 05 — only per-env YAML today. *(missing)*
- [ ] `[nice]` **`SECTION__KEY` env-var overrides** — *(missing)*
- [ ] `[nice]` **Initializers** (`config/initializers/*`) — *(missing)*
- [ ] `[nice]` **Resolve YAML-vs-TOML drift** (pick one) — decision open in ARCHITECTURE.md. *(partial)*

## CLI & Generators — `doido-generators`

- [ ] `[nice]` **Interactive console REPL** (`doido console`) — placeholder. *(partial)*
- [ ] `[nice]` **`generate resource` target** — *(missing)*
- [ ] `[nice]` **`generate` destroy/undo** (`rails destroy`) — *(missing)*
- [ ] `[nice]` **`runner` + `dbconsole` commands** — *(missing)*
- [ ] `[nice]` **i18n locale-files generator** — *(missing)*

## Platform & Deployment (Rails 8 defaults)

- [ ] `[core]` **Asset pipeline** (Propshaft analogue) — *(missing)*
- [ ] `[core]` **Built-in testing story** (fixtures, integration/system test helpers) — only `TestDb`. *(partial)*
- [ ] `[nice]` **Import maps / JS bundling** — *(missing)*
- [ ] `[nice]` **Hotwire** (Turbo + Stimulus) analogue — *(missing)*
- [ ] `[nice]` **i18n framework + locale files** — *(missing)*
- [ ] `[nice]` **Health-check endpoint** (`/up`) — *(missing)*
- [ ] `[nice]` **Production Dockerfile + Kamal deploy + Thruster + Devcontainer** — dev `docker-compose.yml` only. *(partial)*
- [ ] `[nice]` **Solid Queue/Cache/Cable parity** (DB-backed defaults) — db job backend exists; cable/cache db backends missing. *(partial)*

## Integrations (opt-in / vNext)

- [ ] `[deferred]` **Kafka producers/consumers** (`#[consumer]`/`#[topic]`) — ~52-line worktree skeleton. *(partial)*
- [ ] `[deferred]` **MCP server + client** (`#[tool]`/`#[resource]`, HTTP+SSE, OAuth2.1) — ~216-line worktree WIP. *(partial)*
