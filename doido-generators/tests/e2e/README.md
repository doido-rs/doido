# doido-generators release e2e

Final gate before publishing: each scenario scaffolds a SQLite app via the real
`doido-generators` CLI, runs migrations, boots the generated server, and drives a
real interaction (HTTP, WebSocket unit test, or CLI smoke).

## Run locally

```sh
make release-e2e
# or
CARGO_TARGET_DIR=target/e2e-cargo cargo test -p doido-generators --test e2e -- --ignored --nocapture --test-threads=1
```

Set `E2E_KEEP=1` to preserve generated apps under `target/e2e/apps/` for debugging.

## Shared build cache

- Baseline apps are cached under `target/e2e/apps/_base/`.
- Each scenario forks the baseline into `target/e2e/apps/<scenario>/`.
- All builds share `CARGO_TARGET_DIR=target/e2e-cargo` so framework crates link once.

## Validators (every scenario)

1. **Server** — `doido server` on the generated app binary.
2. **Real interaction** — HTTP request, cable `cargo test`, or CLI/file smoke.
3. **Migrations** — `doido db create`, `doido db migrate`, and `doido db migrate status` with no pending migrations.

## Scenario matrix

| Module | Generator / flag |
|--------|------------------|
| `new_baseline` | `doido new --database=sqlite` |
| `new_cable` | `new --cable` + `generate channel` |
| `scaffold_api` | `scaffold --api` |
| `scaffold_html` | `scaffold` (HTML forms) |
| `resource` | `generate resource` |
| `model_fields` | field specs (`unique`, `index`, `references`, …) |
| `migration_add_remove` | `generate migration` add/remove |
| `storage_install` | `storage:install` |
| `storage_adapter` | `storage:adapter` (in `smoke_generators`) |
| `smoke_generators` | `job`, `mailer`, `locale`, `templates`, `generator`, `controller` |
| `kitchen_sink` | all generators in one app |
| `auth_install` | `doido new --auth` (adds `doido-auth` dep + runs install; backlog US-113) |

## CI

The GitHub **Release** workflow runs `make release-e2e` before publishing to
crates.io. PR CI keeps the fast `cargo test --workspace` gate only.
