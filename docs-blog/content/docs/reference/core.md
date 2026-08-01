+++
title = "Core"
description = "Inflections, the error model, logging, environment, and shared utilities that every other crate builds on."
weight = 1
aliases = ['/docs/guides/core/']

+++

> **Design spec:** [`docs/11-core.md`](https://github.com/doido-rs/doido/blob/master/docs/11-core.md).
> This guide documents the API as implemented in `doido-core`.

**Rails analogue: Active Support.** `doido-core` is the foundation every other Doido
crate depends on — and it depends on none of them. It provides the string inflector,
the application error model, logging setup, environment detection, and a handful of
ergonomic extensions (string/time helpers, notifications, a test clock). It also
re-exports the ecosystem crates the whole framework shares so downstream crates can
depend on `doido-core` alone.

## At a glance

```rust
use doido::Result;
use doido::core::{Inflector, Environment, init_logger, load_inflections};

// Re-exported for downstream crates (depend on doido-core, not these directly):
use doido::core::{anyhow, async_trait, serde, thiserror, tracing};
```

## String inflection

`Inflector` is a static facade offering every Rails-style transformation. It powers the
generators (table names, class names, route helpers) and is available anywhere in your
app.

```rust
use doido::core::Inflector;

Inflector::pluralize("post");        // "posts"
Inflector::singularize("comments");  // "comment"
Inflector::camelize("blog_post");    // "BlogPost"
Inflector::camelize_lower("blog_post"); // "blogPost"
Inflector::underscore("BlogPost");   // "blog_post"
Inflector::dasherize("blog_post");   // "blog-post"
Inflector::humanize("author_id");    // "Author"
Inflector::tableize("BlogPost");     // "blog_posts"
Inflector::classify("blog_posts");   // "BlogPost"
Inflector::foreign_key("Author");    // "author_id"
Inflector::constantize("blog_post"); // "BlogPost"
```

## Custom inflection rules

Defaults cover standard English. Override or extend them for irregular plurals,
uncountable nouns, and acronyms — either from a YAML file or programmatically. This is
the same mechanism the generators honour, so `config/inflection.yaml` keeps generated
names consistent with your domain language.

Load rules from `config/inflection.yaml` at boot (returns `Ok(false)` when the file is
absent, so it is safe to call unconditionally):

```rust
// Loads config/inflection.yaml if present; Ok(true) when applied, Ok(false) when missing.
doido::core::load_inflections("config/inflection.yaml")?;
```

```yaml
# config/inflection.yaml
irregular:
  - [person, people]
  - [mouse, mice]
uncountable:
  - equipment
  - information
acronym:
  - API
  - HTTP
```

Or configure rules in code with `init_inflections`, using the `Inflections` builder
(`irregular`, `uncountable`, `acronym`, `plural`, `singular`):

```rust
doido::core::init_inflections(|i| {
    i.irregular("person", "people");
    i.uncountable("equipment");
    i.acronym("API");
});
```

## The error model

Application code uses a single `Result<T>` alias built on `anyhow`; individual crates
define their own typed errors with `thiserror`. `anyhow`, `bail`, and the `Context`
trait (`AnyhowContext`) are re-exported so `?` and error context work out of the box.

```rust
use doido::Result;
use doido::core::anyhow::Context;

fn load_settings() -> Result<Settings> {
    let raw = std::fs::read_to_string("config/settings.toml")
        .context("settings file is missing")?;      // add context to any error
    let settings = toml::from_str(&raw)?;            // ? converts into anyhow::Error
    Ok(settings)
}
```

For a library crate, define a typed error with the re-exported `thiserror`:

```rust
use doido::core::thiserror;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("key not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

## Logging

A single call installs the global `tracing` subscriber. Levels, output format, filter
directives, and file redirection come from the `logger` section of `config/<env>.yml`
via `LoggerConfig`; `init_logger()` uses `RUST_LOG` (or sane defaults) and is idempotent.

```rust
// Simplest form — honours RUST_LOG, falls back to sensible defaults.
doido::core::init_logger();

// Or drive it from config (config/<env>.yml → [logger]):
use doido::core::LoggerConfig;
let cfg = LoggerConfig { level: "debug".into(), ..Default::default() };
doido::core::logger::init_with_config(&cfg);
```

```yaml
# config/development.yml
logger:
  level: debug          # trace | debug | info | warn | error
  format: verbose       # compact (default) | verbose | json_response
  sql: true             # log each SQL statement
  # file: log/development.log   # redirect output (ANSI disabled)
  # directives: "info,my_app=debug,sqlx=warn"  # full EnvFilter override
```

`LogFormat` selects the renderer: `Compact` (single-line, default), `Verbose`
(multi-line with every field and source location), and `JsonResponse` (one JSON object
per HTTP response — access logs and latency metrics).

## Environment

`Environment` distinguishes `Development`, `Test`, and `Production`, resolved from the
process environment.

```rust
use doido::core::Environment;

match Environment::get_env() {
    Environment::Development => { /* verbose errors, hot reload */ }
    Environment::Test => { /* in-memory backends */ }
    Environment::Production => { /* strict, cached */ }
}

let name = Environment::get_env().as_str(); // "development" | "test" | "production"
```

## String & time helpers

`core_ext` adds ergonomic extension traits (`Blank` for emptiness checks, `StringExt`
for common transforms), and `time_ext` offers Rails-like relative-time helpers.

```rust
use doido::core::core_ext::Blank;
use doido::core::time_ext::{days_ago, beginning_of_day};

"".is_blank();                 // true
"  ".is_blank();               // true (whitespace-only)
let since = days_ago(7);       // DateTime<Utc> seven days ago
let start = beginning_of_day(chrono::Utc::now());
```

## Instrumentation & notifications

`trace` provides thin, consistent structured-event helpers used across the framework
(requests, jobs, queries, mail), and `notifications` offers a lightweight pub/sub for
in-process instrumentation.

```rust
use doido::core::notifications::Notifications;

let notifications = Notifications::new();
notifications.subscribe("post.created", |payload| {
    tracing::info!(%payload, "a post was created");
});
notifications.instrument("post.created", "{\"id\":1}");
```

## Test clock

`TestClock` freezes and advances time in tests, so time-dependent logic is
deterministic.

```rust
use doido::core::test_time::TestClock;

let clock = TestClock::new(chrono::Utc::now());
clock.travel(chrono::Duration::hours(2)); // jump forward
let later = clock.now();
```

## See also

- [Configuration](@/docs/reference/configuration.md) — where `logger` and other settings come from.
- [Models](@/docs/reference/models.md) — `sql` logging flows into sea-orm query logs.
- [Generators & CLI](@/docs/reference/generators.md) — consumers of the inflector.
