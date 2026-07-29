# doido-config — Spec

Rails analogue: **Rails.application.config + credentials**

> **Implementation status (2026-07-28) — reconciled.** The framework ships **per-env
> YAML** (`config/<env>.yml`), *not* the layered TOML described below (decision US-085).
> **Implemented:** env-specific YAML loading (`YamlConfig`, folded into `doido-controller`
> + `doido-model`), `SECTION__KEY` env-var overrides (`doido_controller::env_override`),
> and an initializers boot registry. **Deferred (vNext):** the layered TOML design,
> AES-256-GCM encrypted credentials (`credentials.toml.enc` + `master.key`), and
> `doido credentials edit/show` (currently a log-only stub). Everything below describes
> that deferred design — kept as the future target, not current behavior. See
> [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved in interview)

- **File format: TOML**
- **Secrets: encrypted credentials file + env vars, env vars always win**

## Layering Order (lowest → highest priority)

```
1. config/doido.toml            ← base config (all environments)
2. config/doido.<env>.toml      ← environment override (dev/test/prod)
3. config/credentials.toml.enc  ← encrypted secrets (decrypted at boot)
4. Environment variables         ← always override everything
```

`DOIDO_ENV` selects the environment (default: `development`).
`DOIDO_MASTER_KEY` (or `config/master.key`) decrypts the credentials file.

## File Structure Convention

```
config/
  doido.toml                 # base — shared across all envs
  doido.development.toml     # dev overrides
  doido.test.toml            # test overrides
  doido.production.toml      # prod overrides
  credentials.toml.enc       # encrypted secrets (committed to git)
  master.key                 # decryption key (NOT committed, in .gitignore)
```

## Example `config/doido.toml`

```toml
[server]
port = 3000
bind = "127.0.0.1"

[database]
url = "sqlite://db/development.sqlite3"
pool_size = 5

[view]
engine = "tera"
templates_dir = "views"
layout = "application"
hot_reload = true

[log]
level = "info"
```

## Example `config/doido.production.toml`

```toml
[server]
bind = "0.0.0.0"

[database]
pool_size = 20

[view]
hot_reload = false

[log]
level = "warn"
```

## Credentials (`config/credentials.toml.enc`)

Encrypted with `DOIDO_MASTER_KEY`. Decrypted content is plain TOML:

```toml
[database]
url = "postgres://user:pass@host/db"

[mailer]
smtp_password = "secret"

secret_key_base = "abc123..."
```

Edit via CLI: `doido credentials:edit` (opens decrypted file in `$EDITOR`, re-encrypts on save).

## Env Var Mapping

Env vars override any key using double-underscore path notation:

```
DATABASE__URL=postgres://...   →  config.database.url
SERVER__PORT=8080              →  config.server.port
LOG__LEVEL=debug               →  config.log.level
```

## Access Pattern

```rust
use doido_config::Config;

let config = Config::load()?;      // called once at boot
let port = config.server.port;     // typed struct access
let db_url = config.database.url;  // from credentials or env var
```

Config is immutable after load; shared via `Arc<Config>` injected into `Context`.

## `doido-config` Typed Structs

```rust
pub struct Config {
    pub server:   ServerConfig,
    pub database: DatabaseConfig,
    pub view:     ViewConfig,
    pub log:      LogConfig,
    // user-defined sections via serde flatten
}
```

## Known Requirements

- TOML parsing via `toml` crate + `serde`
- Layer merge: base → env file → credentials → env vars
- Encrypted credentials: AES-256-GCM, key from `DOIDO_MASTER_KEY` or `config/master.key`
- Env var override: `SECTION__KEY` double-underscore notation
- `Config::load()` called once at boot; returns `Arc<Config>`
- CLI command `doido credentials:edit` for managing secrets

## TDD Surface

- Test base config loads correctly from TOML
- Test environment file overrides base values
- Test credentials file decrypts and merges correctly
- Test env var overrides take highest precedence
- Test missing `master.key` with no `DOIDO_MASTER_KEY` returns clear error
- Test unknown env var format is ignored gracefully
- Test `Config` struct deserializes all sections correctly
- Test `Config::load()` in test env uses `config/doido.test.toml`
