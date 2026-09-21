# doido-config — Spec

Rails analogue: **Rails.application.config + credentials**

> **Implementation status — reconciled.** The framework ships **per-env YAML**
> (`config/<env>.yml`) loaded by `YamlConfig` (folded into `doido-controller` +
> `doido-model`), **Tera-rendered `get_env` env-var references** inside those YAML
> files (`doido_core::config::render`), an initializers boot registry, and
> **AES-256-GCM encrypted credentials** (`config/credentials.yml.enc` +
> `config/master.key`/`DOIDO_MASTER_KEY`) with the `doido credentials edit/show` CLI.
> This spec describes that design. See [ARCHITECTURE.md](ARCHITECTURE.md).

## Decisions (resolved)

- **File format: per-env YAML** (`config/<env>.yml`). A base-then-env layered format
  (e.g. TOML) was considered and dropped (decision US-085) — per-env YAML is the path.
- **Env vars enter typed config only via `get_env` in YAML** — every
  `config/<env>.yml` is Tera-rendered before parse (`doido_core::config::render`).
  Use `{{ get_env(name="VAR", default="…") }}` for secrets and deploy overrides.
- **Secrets: encrypted credentials file** (`credentials.yml.enc`); bootstrap keys
  (`DOIDO_ENV`, `DOIDO_MASTER_KEY`) and `RUST_LOG` are read outside YAML.
- **App `settings:` blocks stay in the application** — typed `Settings::init` runs in
  `main` or via `Doido::before_run`, not in `doido-core`.

## Environment selection

`DOIDO_ENV` selects the environment (default: `development`). It picks the config file
`config/<env>.yml` and the app's runtime mode. (This is `Environment::get_env()` in
`doido-core` — the environment *mode* selector — not the YAML `get_env` function below.)

## Load order (lowest → highest priority)

```
1. config/<env>.yml            ← per-environment config; env-var values are pulled in
                                  here via `{{ get_env(name="…") }}` (Tera-rendered)
2. config/credentials.yml.enc  ← encrypted secrets (decrypted at boot)
```

Configuration is centralized in `config/<env>.yml`. Environment variables referenced
with `get_env` are substituted at load time; there is no separate post-parse env
override layer for YAML keys.

`DOIDO_MASTER_KEY` (or `config/master.key`) decrypts the credentials file — a bootstrap
key, not a config value.

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
  url: '{{ get_env(name="DATABASE_URL", default="sqlite://db/development.sqlite3") }}'
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
  url: '{{ get_env(name="DATABASE_URL") }}'
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

## Bootstrap env vars (outside YAML)

| Variable | Effect |
|----------|--------|
| `DOIDO_ENV` | Selects `config/<env>.yml` (`development`, `test`, `production`) |
| `RUST_LOG` | Overrides logger verbosity from `logger.level` / `logger.directives` |
| `DOIDO_MASTER_KEY` | Decrypts `config/credentials.yml.enc` |

All other deployment values (`DATABASE_URL`, storage endpoints, mailer SMTP, …) should
be referenced from YAML with `get_env`. Development: copy `.env.example` to `.env`;
generated apps call `startup::prepare()` so `get_env` sees those vars when
`DOIDO_ENV=development`.

## App boot hook

Generated apps call `startup::prepare()` then `doido::Doido::new()…run()`. App-owned
`Settings::init` (for a custom `settings:` YAML block) belongs in `main` or
`.before_run(|| Settings::init())` on the builder — before CLI subcommands that need it.

Integration tests can call `doido::install_test_runtime_globals(conn)` to install the
global pool plus i18n and storage without booting the full CLI.

## Deploy render (`doido config render`)

When a pipeline materializes config before boot (no Tera in the running artifact), each
source file can be rendered through Tera
(`doido_core::config::render`) before it is parsed. A single function, `get_env`, pulls
values from the process environment into the template:

```yaml
server:
  port: {{ get_env(name="PORT", default="3000") }}
database:
  url: '{{ get_env(name="DATABASE_URL") }}'
```

- `get_env(name="VAR")` substitutes the value of the `VAR` environment variable.
- `get_env(name="VAR", default="…")` supplies a fallback when `VAR` is unset.
- A referenced variable that is **unset with no `default`** fails the config load with
  an error naming the variable.

CLI: `doido config render --env production [-o config/production.yml]`.

Arguments are named (Tera functions take keyword arguments); positional
`get_env("VAR")` is not valid. Rendering runs with autoescaping off, so values such as
URLs and connection strings are substituted verbatim.

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
- Env-var references: `{{ get_env(name="VAR", default="…") }}` rendered via Tera before
  YAML parsing; missing var without a default is an error.
- `doido credentials edit/show` for managing secrets.

## TDD surface

- Test per-env YAML loads correctly and deserializes all sections.
- Test `get_env` substitutes an env var's value, uses `default` when unset, and errors
  when unset with no default.
- Test credentials encrypt/decrypt round-trip; wrong master key fails to decrypt.
- Test `credentials show` prints what `credentials edit` saved.
- Test missing `master.key` with no `DOIDO_MASTER_KEY` returns a clear error.
- Test a `get_env` value containing YAML-special characters round-trips (no escaping).
