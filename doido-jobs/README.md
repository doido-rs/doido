# doido-jobs

Background job queues for Doido (Active Job analogue): pluggable memory / database /
Redis backends, a worker engine with leasing and retries, and the `#[job]` macro for
handler registration.

## Running jobs from the CLI

| Rails | Doido |
|-------|-------|
| `perform_later` | `cargo doido jobs run <name> --later` (enqueue; worker processes) |
| `perform_now` | `cargo doido jobs run <name>` (runs in the CLI process) |
| Job processor | `cargo doido worker` / `cargo doido worker --once` |

```bash
# Handlers linked into this binary (sorted)
cargo doido jobs list

# Run in-process — same JobRegistry + JobContext as the worker; no doido_jobs row
cargo doido jobs run sync_nfse_catalog_daily
cargo doido jobs run sync_municipio_cna_lote --payload '{"ibge_codes":["3304557"]}'

# Enqueue one job for the worker
cargo doido jobs run sync_municipio_cna_coordinator --later

### Bulk enqueue (one db `INSERT` per chunk)

The db/redis/memory backends batch only when **many payloads are enqueued in one call**:
`enqueue_jobs`, `{job}_enqueue_many`, or `ctx.enqueue_jobs(...)` from a handler (worker registers
the queue on [`JobContext`](src/context.rs)). Calling `{job}_enqueue` / `.perform_later()` in a
loop still produces **one SQL round-trip per job** (each call is a separate `enqueue_many` of size 1).

# Drain ready jobs after enqueue
cargo doido worker --once
```

Flags for `jobs run`:

- `--payload` — JSON value deserialized into the handler payload (default `{}`)
- `--queue` — override queue (default: `#[job(queue = "...")]`)
- `--timeout` — per-attempt seconds (default: macro `timeout`)
- `--max-retries` — for `--later` only (default: macro `max_retries`)
- `--later` — enqueue instead of `perform_now`

Exit code `0` on success; `1` on unknown job name, bad JSON, handler error, or timeout.

Dead-letter admin: `cargo doido jobs failed`, `retry`, `discard`.

## Web UI

Enable in `config/<env>.yml` (`jobs.webui.enabled: true`) to mount a tabbed dashboard
at `/doido/jobs` when you run `cargo doido server`. Inspect queues, running jobs,
retry backlog, and dead letters; retry or discard from the **Dead** tab. Set
`username` / `password` in production (HTTP Basic Auth). Requires the `webui` feature
on `doido-jobs` (on by default in the `doido` crate).

### DB backend: quieter logs & failure errors

```yaml
jobs:
  type: db
  db:
    sql_logging: false   # stop tracing every reserve poll on doido_jobs
```

On failure the worker writes the error into the job JSON and the `last_error` column.
Retryable failures become **`Failed`** (scheduled at `run_at` via backoff); exhausted jobs
become **`Dead`**. `doido jobs retry` re-enqueues dead jobs with a new id via
[`JobPayload::requeue_fresh`](src/queue.rs) (preserves `job_name` and retry policy).

## Periodic cron

Configure recurring enqueue under `jobs.cron` in `config/<env>.yml`. The
long-lived worker loads tasks at boot and enqueues registered `#[job]` handlers
on each UTC tick (`schedule`: six-field cron, e.g. `0 0 3 * * *` for 03:00 UTC).
Use `jobs.type: db` (Postgres/MySQL) or `redis` so leader locks dedupe ticks
across multiple worker processes. Cron is disabled in the `test` environment by
default; `worker --once` does not run cron.

```yaml
jobs:
  type: db
  cron:
    tasks:
      - job_name: sync_catalog_daily
        schedule: "0 0 3 * * *"
        payload: {}
```
