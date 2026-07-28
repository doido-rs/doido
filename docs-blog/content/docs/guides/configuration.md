+++
title = "Configuration"
description = "Per-environment YAML config, typed access, and environment-variable overrides."
weight = 2
+++

> **Design spec:** [`docs/05-config.md`](https://github.com/doido-rs/doido/blob/master/docs/05-config.md).
> This guide documents the **implemented** path (per-environment YAML). See the note at
> the end for what the spec defers.

**Rails analogue: `config/`.** Doido reads a single YAML file per environment,
`config/<env>.yml`, and exposes it as strongly-typed settings. Environment variables can
override any value, so secrets and per-deployment tweaks stay out of the file. The config
type lives in `doido-controller`.

## At a glance

```rust
use doido_controller::{Config, YamlConfig, ServerConfig};
use doido_controller::env_override::{apply_env_overrides, from_process_env};
```

## Per-environment YAML

Each environment has its own file under `config/`, selected by `Environment::get_env()`
(`development`, `test`, `production`). The recognized top-level sections are `server`,
`logger`, and `middleware`; other subsystems (cache, jobs, storage, …) read their own
sections from the same file.

```yaml
# config/development.yml
server:
  bind: 0.0.0.0
  port: 3000
logger:
  level: debug
  format: verbose
  sql: true
middleware:
  cors:
    enabled: true
    allowed_origins: ["*"]
    allowed_methods: ["GET", "POST"]
```

## Typed access

`YamlConfig` deserializes the file into typed structs. Load the current environment's
file with `load()` (falling back to defaults when absent via the free `config::load()`),
a specific environment with `load_env()`, or parse a string directly with `from_yaml()`.
Access sections through the `Config` trait: `server()`, `logger()`, `middleware()`.

```rust
use doido_controller::{Config, YamlConfig};

// Load config/<current-env>.yml (e.g. config/development.yml).
let config = YamlConfig::load()?;

let addr = format!("{}:{}", config.server().bind, config.server().port); // "0.0.0.0:3000"
let level = &config.logger().level;                                      // "debug"
let cors_on = config.middleware().cors.enabled;                          // true

// Or never fail — fall back to defaults when the file is missing/invalid:
let config = doido_controller::config::load(); // Box<dyn Config>
```

`ServerConfig` defaults to `0.0.0.0:3000`; `LoggerConfig` defaults to `info` (see
[Core](@/docs/guides/core.md)); `MiddlewareConfig`/`CorsConfig` are disabled unless
enabled.

## Environment-variable overrides

Any setting can be overridden with an env var named `SECTION__KEY` (double underscore):
`SERVER__PORT=4000` sets `server.port`. Values are coerced to bool or number when they
parse, otherwise kept as strings — and a new section is created if it doesn't exist yet.

```bash
# Override at launch — ideal for secrets and per-deployment values:
SERVER__PORT=4000 LOGGER__LEVEL=warn DATABASE__URL=postgres://... doido server
```

Overrides are applied to the parsed config value before typed deserialization:

```rust
use doido_controller::env_override::{apply_env_overrides, from_process_env};

let mut value: serde_json::Value = serde_json::to_value(&raw_config)?;

// Pull SECTION__KEY vars straight from the process environment:
from_process_env(&mut value);

// …or apply an explicit set (handy in tests):
apply_env_overrides(&mut value, &[
    ("SERVER__PORT".into(), "4000".into()),
    ("LOGGER__LEVEL".into(), "debug".into()),
]);
```

## Subsystem configuration

The `server`/`logger`/`middleware` sections are typed by `YamlConfig`, but each pluggable
subsystem reads its own section from the same `config/<env>.yml`. See the relevant guide
for the exact keys:

```yaml
cache:   { type: memory }                 # → Cache guide
jobs:    { backend: memory, queues: [default] }  # → Jobs guide
storage: { service: local }               # → Storage guide
database: { url: sqlite://db/development.db }     # → Models guide
```

## Spec vs. implementation

> The spec describes **layered TOML** (`config/doido.toml` + per-env overrides) and
> **AES-256-GCM encrypted credentials** (`config/credentials.toml.enc` + `master.key`).
> Those are **deferred**. The implemented, tested path is **per-environment YAML** plus
> `SECTION__KEY` env overrides, documented above.

## See also

- [Core](@/docs/guides/core.md) — the `logger` section and `Environment`.
- [Middleware & sessions](@/docs/guides/middleware.md) — the `middleware.cors` section.
- [Models](@/docs/guides/models.md) — the `database` section and connection pool.
