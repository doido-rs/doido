# doido-config — Spec

Rails analogue: **Rails.application.config + credentials**

> **Implementation status — reconciled.** The framework ships **per-env YAML**
> (`config/<env>.yml`) loaded by `YamlConfig` (folded into `doido-controller` +
> `doido-model`), `SECTION__KEY` env-var overrides (`doido_controller::env_override`),
> an initializers boot registry, and **AES-256-GCM encrypted credentials**
> (`config/credentials.yml.enc` + `config/master.key`/`DOIDO_MASTER_KEY`) with the
> `doido credentials edit/show` CLI. This spec describes that design. See
> [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved)

- **File format: per-env YAML** (`config/<env>.yml`). A base-then-env layered format
  (e.g. TOML) was considered and dropped (decision US-085) — per-env YAML is the path.
- **Secrets: encrypted credentials file + env vars, env vars always win.**

## Environment selection

`DOIDO_ENV` selects the environment (default: `development`). It picks the config file
`config/<env>.yml` and the app's runtime mode.

## Load order (lowest → highest priority)

```
1. config/<env>.yml            ← per-environment config
2. config/credentials.yml.enc  ← encrypted secrets (decrypted at boot)
3. Environment variables       ← always override everything
```

`DOIDO_MASTER_KEY` (or `config/master.key`) decrypts the credentials file.

## File structure convention

```
config/
  development.yml            # dev config
  test.yml                  # test config
  production.yml            # prod config
  credentials.yml.enc       # encrypted secrets (committed to git)
  master.key                # decryption key (NOT committed, in .gitignore)
```

## Example `config/development.yml`

```yaml
server:
  bind: "127.0.0.1"
  port: 3000

database:
  url: "sqlite://db/development.sqlite3"
  pool: 5

logger:
  level: "info"
  sql: true

middleware:
  cors:
    enabled: false
    allowed_origins: []
    allowed_methods: []
```

## Example `config/production.yml`

```yaml
server:
  bind: "0.0.0.0"
  port: 3000

database:
  pool: 20

logger:
  level: "warn"
  sql: false
```

## Credentials (`config/credentials.yml.enc`)

Encrypted with AES-256-GCM (`nonce || ciphertext+tag`) via `doido_core::crypto`, keyed by
`config/master.key` or the `DOIDO_MASTER_KEY` env var. The decrypted content is plain YAML:

```yaml
secret_key_base: "abc123..."
database:
  url: "postgres://user:pass@host/db"
mailer:
  smtp_password: "secret"
```

Manage via CLI:

- `doido credentials edit` — decrypts into a temp file, opens `$EDITOR`, re-encrypts on save
  (auto-generates + gitignores `config/master.key` on first run).
- `doido credentials show` — decrypts and prints to stdout.

## Env var mapping

Env vars override any config key using double-underscore path notation
(`doido_controller::env_override`), coerced to bool/number/string:

```
SERVER__PORT=8080     →  config.server.port
DATABASE__URL=...     →  config.database.url
LOGGER__LEVEL=debug   →  config.logger.level
```

## Access pattern

```rust
// Loaded once at boot for the current environment.
let config = doido_controller::YamlConfig::load()?; // implements the Config trait
let port = config.server().port;                    // typed section access
```

Config is immutable after load. The DB pool, view engine, and other singletons are wired
from it during the boot sequence (see ARCHITECTURE.md → “Runtime boot sequence”).

## Typed config structs

```rust
pub struct ServerConfig   { pub bind: String, pub port: u16 }
pub struct DatabaseConfig { pub url: String, pub pool: Option<u32>, pub connect_timeout: Option<u64> }
pub struct LoggerConfig   { pub level: String, pub sql: bool }
pub struct MiddlewareConfig { pub cors: CorsConfig /* … */ }
```

## Initializers

An initializers registry (`doido_controller::initializers`) runs named boot hooks in
order after config load — the place to validate credentials, set up third-party clients,
etc. The first error aborts boot.

## Known requirements

- YAML parsing via `serde` (`serde_norway`).
- Per-env file resolution from `DOIDO_ENV` (`config/<env>.yml`).
- Encrypted credentials: AES-256-GCM, key from `DOIDO_MASTER_KEY` or `config/master.key`.
- Env var override: `SECTION__KEY` double-underscore notation, with type coercion.
- `doido credentials edit/show` for managing secrets.

## TDD surface

- Test per-env YAML loads correctly and deserializes all sections.
- Test env var overrides take highest precedence and coerce types.
- Test credentials encrypt/decrypt round-trip; wrong master key fails to decrypt.
- Test `credentials show` prints what `credentials edit` saved.
- Test missing `master.key` with no `DOIDO_MASTER_KEY` returns a clear error.
- Test unknown env var format is ignored gracefully.
