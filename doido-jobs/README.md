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
