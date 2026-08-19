# doido release e2e

Final gate before publishing: each scenario scaffolds a SQLite app via the real
`doido` CLI, runs migrations, boots the generated server, and drives a real
interaction (HTTP, WebSocket unit test, or CLI smoke).

## Run locally

```sh
make release-e2e
# or
CARGO_TARGET_DIR=target/e2e-cargo cargo test -p doido --test e2e -- --ignored --nocapture --test-threads=1
```

Set `E2E_KEEP=1` to preserve generated apps under `target/e2e/apps/` for debugging.

## Shared build cache

- Baseline apps are cached under `target/e2e/apps/_base/`.
- Each scenario forks the baseline into `target/e2e/apps/<scenario>/` (without copying `target/`).
- All generated apps share `CARGO_TARGET_DIR=target/e2e/apps/cargo-target` so framework crates link once.
- The first scenario per `BaseProfile` warms that profile's baseline build; later scenarios only recompile app changes.
- The e2e test harness itself still uses `target/e2e-cargo` (see `make release-e2e`) so `-D warnings` on generated apps does not affect harness flags.

## Validators (every scenario)

1. **Server** — `doido server` on the generated app binary.
2. **Real interaction** — HTTP request, cable `cargo test`, or CLI/file smoke.
3. **Migrations** — `doido db create`, `doido db migrate`, and `doido db migrate status` with no pending migrations. Migrate re-exports entities into `app/models/_entities/` without overwriting extension stubs in `app/models/<name>.rs`.
4. **Seed crate** — every `doido new` baseline includes `db/seed/` (workspace member with `serde` + `doido` deps); `db_seed` also runs `doido db seed` and asserts rows over HTTP.

## Scenario matrix

| Module | Generator / flag |
|--------|------------------|
| `new_baseline` | `doido new --database=sqlite` |
| `bootstrap_migrations` | bootstrap storage tables (always) and `doido_jobs` (with `--jobs=db` only); asserts `db/seed/` scaffold |
| `db_seed` | `doido db seed` via `db/seed` crate + HTTP index lists seeded row |
| `seed_initial_user` | `doido new --auth` seeds an initial admin user; asserts `users` row + idempotency |
| `db_commands` | `doido db schema dump\|load`, `reset`, `prepare`, `create`, `migrate` on a scaffolded app |
| `new_cable` | `new --cable` + `generate channel` |
| `scaffold_api` | `scaffold --api` |
| `scaffold_html` | `scaffold` (HTML forms) |
| `resource` | `generate resource` |
| `model_fields` | field specs (`unique`, `index`, `references`, …) |
| `model_validations` | `Validate`/`Errors` in extension stubs survive entity re-export on migrate |
| `migration_add_remove` | `generate migration` add/remove |
| `storage_install` | `storage:install` |
| `storage_adapter` | `storage:adapter` (in `smoke_generators`) |
| `smoke_generators` | `job`, `mailer`, `locale`, `templates`, `generator`, `controller` |
| `helper` | `generate helper` + HTTP call through controller |
| `kitchen_sink` | all generators in one app |
| `auth_install` | `doido new --auth` (HTML views + form sign-up/in/out) and `doido new --auth --api` (JSON auth) |

## CI

The GitHub **Release** workflow runs `make release-e2e` before publishing to
crates.io. PR CI keeps the fast `cargo test --workspace` gate only.
