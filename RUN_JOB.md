# Implementation prompt: `doido jobs run` (execute jobs from the CLI)

Use this document as the product/technical brief to implement **direct and asynchronous job execution** in **`doido-jobs`** (and wire it through **`doido` CLI**). It was written from the QuickNFe need to run catalog sync jobs (e.g. `sync_nfse_catalog_daily`, `sync_municipio_cna_lote`) without manually enqueueing rows and starting a long-lived worker.

---

## Problem today

| Capability | Exists? | Notes |
|------------|---------|--------|
| Enqueue via `#[job]` builders (`Job::new(payload).enqueue(queue)`) | Yes | App code only |
| Drain queue | Yes | `cargo doido worker` / `cargo doido worker --once` |
| Dead-letter admin | Yes | `cargo doido jobs failed` / `retry` / `discard` |
| **Run a named job once from CLI** | **Yes** | `cargo doido jobs run` / `perform` (`perform_job_now`) |
| List registered job names | **Yes** | `cargo doido jobs list` |

Applications (QuickNFe) define jobs with `#[job(queue = "…", timeout = …)]`. Handlers are registered at link time via `inventory` and `JobRegistry::from_inventory()`. The worker builds `JobContext` (DB pool, etc.), reserves a row from `doido_jobs`, applies timeout/retry/dead-letter via `finalize_job`.

**Developer friction:** operational tasks (sync catalog, reimport Anexo A, one municipality IBGE) require either writing Rust enqueue code, relying on seed/bootstrap, or inserting JSON into `doido_jobs` by hand.

---

## Goal

Add a **`doido jobs run`** (alias **`perform`**) subcommand that can:

1. **`perform_now` (default)** — Build a synthetic `JobPayload`, dispatch through the same `JobRegistry` and `JobContext` as the worker, **await completion in the CLI process**, exit code 0/1. **Do not** require a pre-enqueued row; **do not** call `ack`/`nack`/`dead_letter` on the queue (no side effects on `doido_jobs` unless explicitly requested).

2. **`perform_later` (optional flag)** — Enqueue one job with the given name/payload/queue (same as app builders), print job id, exit 0. Work runs **asynchronously** when a worker drains the queue (existing behavior).

3. **Discoverability** — `doido jobs list` prints registered `job_name` values from `JobRegistry::names()` (sorted).

This mirrors Rails Active Job **`perform_now`** vs **`perform_later`**, adapted to Doido’s `JobPayload` + registry model.

---

## Proposed CLI (target UX)

```bash
# List handlers linked into this binary
cargo doido jobs list

# Run handler in-process (async perform, await in CLI) — primary ask
cargo doido jobs run sync_nfse_catalog_daily
cargo doido jobs run sync_nfse_catalog_daily --payload '{"skip_reschedule":true}'

cargo doido jobs run sync_municipio_cna_lote \
  --payload '{"ibge_codes":["3304557"]}'

# Override queue/policy for this invocation (optional; defaults from #[job] macro
# are NOT available at runtime unless stored in registry metadata — see below)
cargo doido jobs run import_nfse_anexo_a --queue default

# Enqueue for worker (async via DB/Redis), do not wait
cargo doido jobs run sync_municipio_cna_coordinator --later

# Drain everything ready after enqueue (app-level convenience; optional in doido)
cargo doido worker --once
```

**QuickNFe examples** (from `backend/`, with `.env` loaded):

```bash
cd backend
cargo doido jobs run sync_nfse_catalog_daily --payload '{"skip_reschedule":true}'
cargo doido jobs run sync_municipio_cna_lote --payload '{"ibge_codes":["3304557"]}'
cargo doido jobs run import_nfse_anexo_a
```

---

## Scope (doido-jobs)

### 1. New module `commands/run.rs` (or extend `commands/jobs.rs`)

Implement `JobsCommand::Run(RunArgs)` with clap:

| Flag | Purpose |
|------|---------|
| `job_name` (positional) | Must match `JobPayload::job_name` / `#[job]` fn name (e.g. `sync_municipio_cna_lote`) |
| `--payload <JSON>` | Object deserialized into the handler’s payload type at dispatch time (default `{}` for unit/default payloads) |
| `--queue <name>` | Queue name on synthetic payload (default `default`; apps may override per call) |
| `--later` | If set, enqueue via configured `JobQueue` instead of in-process perform |
| `--timeout <secs>` | Override job timeout for this run (default: sensible global default, e.g. 3600, or future registry metadata) |
| `--max-retries <n>` | For `--later` only; default 0 or macro default |

**Validation:**

- Unknown `job_name` → exit 1, message listing nearest names from `JobRegistry::names()` (optional fuzzy hint).
- Invalid JSON → exit 1.
- Handler error → exit 1, print `error:` chain to stderr; respect `RUST_LOG` / tracing.

### 2. Shared runner: `JobRunner` or `perform_job_now`

Extract logic shared with `WorkerEngine` so behavior stays consistent:

```text
load JobsConfig + build JobQueue (same as worker — pool init for db backend)
build JobContext (insert DB pool when available, same as worker.rs)
registry = JobRegistry::from_inventory()
synthetic JobPayload:
  - new uuid
  - job_name = CLI arg
  - payload = serde_json::Value from --payload
  - queue, max_retries, timeout, backoff from flags / defaults
  - status irrelevant for perform_now

apply timeout: reuse finalize_job’s timeout wrapper BUT skip queue.ack/nack
  OR add perform_now path that only does:
    tokio::time::timeout(duration, registry.dispatch(job, ctx)).await
on success: Ok(()) → exit 0
on Err: print and exit 1
```

**Important:** `perform_now` must **not** leave stray `running` rows in `doido_jobs`. Do not call `reserve`/`enqueue` unless `--later`.

**Panics:** treat like worker (log “job panicked”, exit 1).

### 3. `perform_later` (`--later`)

Build `JobPayload` with `JobPayload::new(queue, payload, max_retries).with_name(job_name).with_timeout(...)`, enqueue on configured backend, print:

```text
enqueued job_id=<uuid> queue=<name> job_name=<name>
```

No wait. Document that user runs `doido worker` or `doido worker --once`.

### 4. `jobs list`

Print one name per line (or table with queue if registry exposes metadata later). Used by scripts and docs.

### 5. Registry metadata (stretch / phase 2)

Today `#[job]` embeds queue, timeout, max_retries at **compile time** only on generated `*Job` builders, not in `JobRegistration`. For CLI defaults without flags:

- **Phase 1:** require `--queue` / `--timeout` or document app-specific defaults in app README.
- **Phase 2 (recommended):** extend `JobRegistration` with optional `default_queue`, `default_timeout`, `default_max_retries` filled by the macro, so `doido jobs run foo` picks the same policy as `FooJob::new(…).enqueue(…)`.

### 6. Wire into `doido` CLI

In `doido-*/src/cli.rs`, `Commands::Jobs { action }` already delegates to `doido_jobs::commands::jobs::run`. Extend `JobsCommand` enum and help text.

Ensure **`perform_now` runs after** `install_runtime_globals` / DB pool init when the jobs backend is `db` (same ordering as `Worker` command — today `before_run` in apps runs before parse; **`jobs run` must init pool** inside the subcommand, mirroring `worker.rs`).

---

## Non-goals (v1)

- Running jobs that are **already reserved** in `doido_jobs` by id (separate: `doido jobs retry` covers dead letters only).
- Cron scheduling (`run_at`) from CLI (use `--later` + existing `at()` on builders from app code, or future `jobs run --at`).
- Cross-binary jobs (handler must be linked into the app binary that runs the CLI — same as worker).

---

## Tests (doido-jobs)

1. **Unit:** `perform_now` with in-memory registry + mock handler; success and Err exit paths; timeout.
2. **Unit:** unknown job name error message includes `list` hint.
3. **Integration:** app test crate with one `#[job]`; `jobs run my_job --payload '{"n":1}'` in subprocess; assert side effect / stdout.
4. **Regression:** `perform_now` does not insert into `doido_jobs` (db backend, count before/after).

---

## Documentation

- Update doido-jobs README: section **Running jobs from the CLI**.
- Mention relation to `worker --once` (queue drain vs inline perform).
- Rails analogy table: `perform_later` → enqueue; `perform_now` → `jobs run`; worker → job processor.

---

## Acceptance criteria

1. `cargo doido jobs list` shows all `#[job]` names in the application binary.
2. `cargo doido jobs run <name>` executes the handler in the CLI process, with DB `JobContext` when configured, honoring timeout.
3. `cargo doido jobs run <name> --later` enqueues exactly one job; worker can process it.
4. `cargo doido jobs run <name> --payload '<json>'` passes deserialized payload to the handler.
5. Exit code 0 on success, non-zero on validation/handler/timeout errors.
6. QuickNFe can sync Rio (`3304557`) with two commands or one run + optional worker:

   ```bash
   cargo doido jobs run sync_municipio_cna_lote --payload '{"ibge_codes":["3304557"]}'
   ```

---

## Implementation checklist (for the agent)

- [x] Extend `JobsCommand` in `doido-jobs/src/commands/jobs.rs` (or split `run.rs`).
- [x] Add `perform_now` helper using `JobRegistry::dispatch` + timeout (no queue ack).
- [x] Add `--later` enqueue path using existing `JobQueue::enqueue`.
- [x] Add `list` subcommand.
- [x] Ensure `build_configured_queue` + `JobContext` + pool init for `db` backend before perform.
- [x] Hook CLI in `doido` crate; update `--help` for `doido jobs`.
- [x] Tests + README.
- [x] (Phase 2) Macro emits default queue/timeout into `JobRegistration` for CLI defaults.

---

## Reference (QuickNFe / current stack)

- Doido version in app: `0.0.32-test2` (`backend/Cargo.toml`).
- Jobs defined under `backend/app/jobs/` (e.g. `sync_nfse_catalog_daily`, `sync_municipio_cna_lote`, `import_nfse_anexo_a`).
- Queues in `config/development.yml`: `default`, `catalog`.
- Catalog sync docs: `docs/nfse/catalog-sync.md`.

After this lands in **doido-jobs**, QuickNFe should only document the commands in `docs/nfse/catalog-sync.md` and remove workarounds (manual enqueue, bootstrap-only seed).
