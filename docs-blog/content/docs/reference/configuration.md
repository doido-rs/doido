+++
title = "Configuration"
description = "Per-environment YAML config, typed access, and environment-variable overrides."
weight = 2
aliases = ['/docs/guides/configuration/']

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
use doido::controller::{Config, YamlConfig, ServerConfig};
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
use doido::controller::{Config, YamlConfig};

// Load config/<current-env>.yml (e.g. config/development.yml).
let config = YamlConfig::load()?;

let addr = format!("{}:{}", config.server().bind, config.server().port); // "0.0.0.0:3000"
let level = &config.logger().level;                                      // "debug"
let cors_on = config.middleware().cors.enabled;                          // true

// Or never fail — fall back to defaults when the file is missing/invalid:
let config = doido::controller::config::load(); // Box<dyn Config>
```

`ServerConfig` defaults to `0.0.0.0:3000`; `LoggerConfig` defaults to `info` (see
[Core](@/docs/reference/core.md)); `MiddlewareConfig`/`CorsConfig` are disabled unless
enabled.

## Environment variables

Each `config/<env>.yml` is **Tera-rendered on load**. Reference environment variables
with `get_env`:

```yaml
database:
  url: '{{ get_env(name="DATABASE_URL", default="sqlite://db/development.db") }}'
```

| Variable | Purpose |
|----------|---------|
| `DOIDO_ENV` | Selects `config/<env>.yml` (`development`, `test`, `production`) |
| `RUST_LOG` | Overrides logger verbosity from `logger.level` / `logger.directives` |
| `DOIDO_MASTER_KEY` | Decrypts `config/credentials.yml.enc` |
| `DOIDO_LOCALE` | Process-default backend locale (`en`, `pt`, `pt-BR`, `pt_BR` → catalog `pt_BR`) |

Deployment secrets (`DATABASE_URL`, SMTP, storage endpoints, …) belong in YAML via
`get_env`, not as implicit framework overrides. Use `.env` in development
(`startup::prepare()`); inject env in production so render sees the values.

Optional bake-at-deploy: `doido config render --env production -o config/production.yml`.

App `settings:` and `Doido::before_run(|| Settings::init())`; tests:
`doido::install_test_runtime_globals(conn)`.

## Subsystem configuration

The `server`/`logger`/`middleware` sections are typed by `YamlConfig`, but each pluggable
subsystem reads its own section from the same `config/<env>.yml`. See the relevant guide
for the exact keys:

```yaml
cache:   { type: memory }                 # → Cache guide
jobs:    { backend: memory, queues: [default] }  # → Jobs guide
storage: { driver: local }               # → Storage guide
database: { url: sqlite://db/development.db }     # → Models guide
```

## Spec vs. implementation

> Layered TOML was dropped (US-085). The path is **per-environment YAML** with Tera
> `get_env`, optional `doido config render`, and **AES-256-GCM credentials**
> (`credentials.yml.enc` + `master.key`).

## See also

- [Core](@/docs/reference/core.md) — the `logger` section and `Environment`.
- [Middleware & sessions](@/docs/reference/middleware.md) — the `middleware.cors` section.
- [Models](@/docs/reference/models.md) — the `database` section and connection pool.
