# Changelog — doido-jobs

All notable changes to `doido-jobs` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every crate in the `doido` workspace shares one version; the release process is
documented in `RELEASING.md` at the repository root.

> Background jobs, queue backends, and retries (Active Job analogue).

## Unreleased

### Added

- **`JobQueue::cancel_job`**: remove pending, running, or retry-scheduled jobs (not
  dead-letter) on memory, db, and redis backends.
- Jobs Web UI **Enqueue** tab (`POST {mount}/enqueue`): pick a registered handler,
  optional JSON payload (object or array to submit many jobs at once), queue /
  timeout / max-retries overrides; enqueue now, schedule at UTC, or perform now
  in-process.
- Jobs Web UI **bulk actions** on listing tabs: delete selected on In queue /
  Executing / Retry; retry or discard selected and discard all on **Dead**
  (`POST {mount}/bulk`).
- Jobs Web UI **Live poll** toggle: refresh the current tab every 10 seconds
  (preference stored in `localStorage`).
- Dashboard **relative timestamps** (e.g. “5 minutes ago”) with full datetime
  tooltips.
- **`JobQueue::enqueue_many` / `enqueue_many_at`** on all backends, chunked by
  `jobs.enqueue_batch_size`; `JobContext::enqueue_jobs`, `{Name}_enqueue_many`
  from `#[job]`, and batched `perform_jobs_now` for in-process runs.

### Changed

- **`cargo doido jobs run`**: clearer errors when enqueue or perform fails.

## 0.0.32-test4 - 2026-10-30

### Performance

- Jobs Web UI: `queue_counts` only on the **Queues** tab; optional
  `jobs.webui.stats_ttl_secs` in-process cache; `include_payload` query param
  (payload omitted by default).
- **DbQueue:** aggregated SQL for `queue_counts`; lighter dashboard `list_jobs`
  projection; cheaper cursor “previous page” hints.
- **RedisQueue:** batched `MGET` for job loads; single processing-set scan for
  running counts; chunked ready-set reads for listings.
- **MemoryQueue:** paginate from sort keys, then materialize only jobs on the
  current page.

### Changed

- Queue backends live under `doido-jobs/src/drivers/`; public imports
  (`doido_jobs::{db,memory,redis}`) unchanged.
- `JobQueue::list_jobs` accepts a `light` flag to skip heavy payload fields for
  dashboard lists.

## 0.0.32-test3 - 2026-10-30

### Added

- CLI **`cargo doido jobs run`**: run a handler in-process (`perform_now`) or enqueue
  with `--later`; supports `--payload`, `--queue`, `--timeout`, and `--max-retries`.
- **`perform_job_now`**: execute a registered job in the current process without
  writing queue rows.
- **Periodic cron** (`jobs.cron` in `config/<env>.yml`): worker enqueues registered
  jobs on UTC cron ticks at boot; leader locks on `db` / `redis` backends dedupe
  ticks across processes.
- **Jobs Web UI** (`jobs.webui`, `webui` feature): Mission Control–style dashboard
  mounted from `cargo doido server` (default `/doido/jobs`); queue counts, in-queue,
  executing, retry, and dead tabs; per-job and bulk dead-letter actions; optional
  HTTP Basic Auth in production.
- **Cursor pagination** on dashboard tables (`page_token`, `queue_page_token`) instead
  of offset/limit listing.
- Introspection API: `JobQueue::list_jobs` returns a token-paginated `JobListPage`;
  `JobBucket::InQueue` for the dashboard’s combined ready queue view.

### Changed

- **DbQueue** on MySQL/PostgreSQL: parallel reserve via `SELECT … FOR UPDATE SKIP
  LOCKED` (within a single worker process).

## 0.0.32-test2 - 2026-10-15

### Fixed

- `DbQueue::migrate` creates the `doido_jobs` `(queue, status, run_at)` index with
  `IF NOT EXISTS`, so a second migrate no longer fails when the table and index
  are already present.

## 0.0.32-test1 - 2026-10-14

_No user-facing changes yet._

## 0.0.32 - 2026-10-13

### Changed

- Database-backed queue (`DbQueue`) reads and writes through a SeaORM entity and
  `doido-model` migration helpers instead of raw SQL.
- `doido_jobs` uses an auto-increment integer primary key (`id`); the worker-facing
  job identifier is stored in a unique `job_id` column. `JobQueue` methods still
  take the same string job IDs as before.

## 0.0.24 - 2026-08-14

- Baseline changelog. Releases up to and including `0.0.24` are recorded only in
  the git tags (`v0.0.*`) and their GitHub Releases; per-crate history starts here.
