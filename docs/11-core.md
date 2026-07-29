# doido-core — Spec

Rails analogue: **Active Support**

## Decisions (resolved in interview)

- **Error strategy: `thiserror` per crate, `anyhow` at app level**
  - Each crate owns a typed error enum via `thiserror`
  - All implement `std::error::Error`, so `?` propagates into `anyhow::Error` in app code
  - `doido-core` defines no umbrella error — only a `Result<T>` alias using `anyhow`
- **All string inflection transformations ship** — plus a custom inflection rules file

## Error Convention Per Crate

Each crate defines its own error type:

```rust
// Pattern every crate follows
#[derive(thiserror::Error, Debug)]
pub enum RouterError { ... }

#[derive(thiserror::Error, Debug)]
pub enum ControllerError { ... }

#[derive(thiserror::Error, Debug)]
pub enum ModelError { ... }

#[derive(thiserror::Error, Debug)]
pub enum ViewError { ... }

// etc.
```

`doido-core` re-exports `anyhow` and `thiserror` for convenience so crates only depend on `doido-core`:

```rust
pub use anyhow::{self, anyhow, bail, Context as AnyhowContext};
pub use thiserror;

/// App-level result type — used in controllers, jobs, and application code
pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;
```

## Inflector — All Transformations

```rust
use doido_core::inflector::Inflector;

// Standard transformations
Inflector::pluralize("post")           // → "posts"
Inflector::pluralize("person")         // → "people"
Inflector::singularize("posts")        // → "post"
Inflector::camelize("post_comment")    // → "PostComment"
Inflector::camelize_lower("post_comment") // → "postComment"
Inflector::underscore("PostComment")   // → "post_comment"
Inflector::dasherize("post_comment")   // → "post-comment"
Inflector::humanize("post_comment")    // → "Post comment"
Inflector::tableize("PostComment")     // → "post_comments"
Inflector::classify("post_comments")   // → "PostComment"
Inflector::foreign_key("PostComment")  // → "post_comment_id"
Inflector::constantize("post_comment") // → "POST_COMMENT"
```

Used by:
- `doido-generators` — derives file names, table names, module names from user input
- `doido-router` — derives route names from controller names
- `doido-cli` — `doido routes` table formatting

## Custom Inflection Rules — `config/inflections.rs`

> **Implementation status (2026-07-28) — reconciled.** The implemented mechanism is a
> **runtime `config/inflection.yaml`**, not the compiled `config/inflections.rs` shown
> below. Functionally equivalent (irregulars, uncountables, custom plural/singular rules,
> acronyms); the compiled-Rust form remains an unadopted alternative. See
> [ARCHITECTURE.md](ARCHITECTURE.md).

Users override or extend the default rules in `config/inflections.rs`:

```rust
// config/inflections.rs
use doido_core::inflector::Inflections;

pub fn configure(inflections: &mut Inflections) {
    // Override irregular singular/plural
    inflections.irregular("person", "people");
    inflections.irregular("goose", "geese");

    // Uncountable words (same singular and plural)
    inflections.uncountable("sheep");
    inflections.uncountable("fish");
    inflections.uncountable("money");

    // Custom plural rules (regex pattern, replacement)
    inflections.plural(r"(quiz)$", "${1}zes");

    // Custom singular rules
    inflections.singular(r"(quiz)zes$", "${1}");

    // Acronyms (preserved in camelize/underscore)
    inflections.acronym("API");
    inflections.acronym("HTML");
    inflections.acronym("HTTP");
}
```

`config/inflections.rs` is loaded at boot by the framework. `doido-generators` generates this file as part of `doido new <app>`.

## Module Structure

```
doido-core/
  src/
    lib.rs
    error.rs          ← Result<T> alias, re-exports thiserror + anyhow
    inflector/
      mod.rs           ← Inflector struct + all transformation methods
      rules.rs         ← default English inflection rules
      inflections.rs   ← Inflections config struct (user-facing)
    async_trait.rs    ← re-export async_trait for convenience
    logger.rs         ← centralized tracing_subscriber setup (logger::init)
    trace.rs          ← tracing event helpers used across crates
```

## Logging (centralized)

`doido_core::logger` owns the framework's `tracing_subscriber` setup — the single
place logging is configured. `doido server` reads the `logger` section of
`config/<env>.yml` into `logger::LoggerConfig` and calls
`logger::init_with_config(&config.logger)` at boot, after which everything flows
through one subscriber:

- HTTP **requests & responses** — logged at `INFO` by the always-on `TraceLayer`
  in `doido-controller`'s middleware stack (method, path, status, latency).
- **ORM queries** — sea-orm's SQL logging (toggled by `logger.sql`, enabled on
  the connection in `doido-model`) emits under target `sqlx::query` at `INFO`.
- Jobs, mail, custom events — the [Tracing Helpers](#tracing-helpers) below.

The `logger` config section drives all of it:

```yaml
logger:
  level: debug          # app log level → EnvFilter (info|debug|warn|…)
  # directives: info,my_app=debug,sqlx=warn   # full EnvFilter override
  file: log/test.log    # redirect output to a file (appended, no ANSI); omit for stdout
  sql: true             # sea-orm SQL statement logging
  format: compact       # compact | verbose | json_response
```

`level` is combined with the framework's `NOISE_DIRECTIVES` (so SQL/HTTP
internals stay quiet); `directives`, when set, fully replaces it. Because
sea-orm logs through this same subscriber, setting `file` captures SQL too.
`RUST_LOG` (`EnvFilter` syntax), when set, overrides the configured verbosity;
when no config file is present, `logger::DEFAULT_DIRECTIVES` applies (`info` +
`sqlx::query=info`, with pool and hyper/tower internals quieted).
`init`/`init_with_config` are idempotent.

`format` selects the renderer:

- **`compact`** (default) — single-line human-readable events.
- **`verbose`** — pretty, multi-line output with every field plus thread and
  source location; for inspecting all the structured data in development.
- **`json_response`** — one JSON object per HTTP **response** event (status,
  `latency_ms`, `request_id`, …), suppressing everything else. The request line
  and app logs are filtered out by isolating the `doido::response` target
  (`RESPONSE_TARGET`); suited to access logs and latency metrics. An explicit
  `directives` or `RUST_LOG` widens it if needed.

## Tracing Helpers

Thin wrappers so crates emit consistent structured events without duplicating setup:

```rust
doido_core::trace::request(method, path, status, latency_ms);
doido_core::trace::job(job_name, queue, attempt, result);
doido_core::trace::query(sql, duration_ms);
doido_core::trace::mail(to, subject, deliverer);
```

HTTP logging proper is emitted by `doido-controller`'s `logging::log_requests`
middleware, which assigns each request a `request_id` (UUID, or the inbound
`x-request-id` header) shared by two lines: a `request` line (method, path,
query, request headers) and a `response` line (status, latency, response
headers). Sensitive headers are redacted, and the id is echoed back on the
`x-request-id` response header.

## Known Requirements

- `doido-core` is a **leaf dependency** — depends on nothing else in the workspace
- Re-exports: `anyhow`, `thiserror`, `async_trait`, `serde`, `tracing`
- `Result<T>` alias is `anyhow::Result<T>` — used in app-level code
- All inflection transformations implemented and tested for English by default
- Custom inflection file loaded at app boot; overrides default rules
- Acronym support in `camelize` / `underscore` (e.g. `APIClient` ↔ `api_client`)

## TDD Surface

- Test each inflection transformation with standard English cases
- Test irregular overrides (`person` → `people`)
- Test uncountable words return same form for singular and plural
- Test custom plural/singular regex rules apply correctly
- Test acronym preservation in `camelize` and `underscore`
- Test `config/inflections.rs` overrides take precedence over defaults
- Test `tableize` and `classify` are inverses of each other
- Test `foreign_key` output matches expected convention
- Test `Result<T>` propagates `?` from a `thiserror` crate error into `anyhow::Error`
- Test tracing helpers emit events with correct structured fields
