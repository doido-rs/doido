# Changelog — doido-jobs

All notable changes to `doido-jobs` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every crate in the `doido` workspace shares one version; the release process is
documented in `RELEASING.md` at the repository root.

> Background jobs, queue backends, and retries (Active Job analogue).

## Unreleased

<<<<<<< HEAD
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
=======
- DbQueue (MySQL/PostgreSQL): parallel job reserve via `FOR UPDATE SKIP LOCKED`
- **Performance:** Jobs Web UI — `queue_counts` only on the Queues tab; optional `jobs.webui.stats_ttl_secs`; `include_payload` query param (payload omitted by default); lighter DB `list_jobs` and aggregated SQL `queue_counts`; Redis batched `MGET` and chunked ready-set scans; memory driver lists pages without cloning full buckets
- **Changed:** queue backends moved to `doido-jobs/src/drivers/` (public `doido_jobs::{db,memory,redis}` re-exports unchanged)
- **Changed:** `JobQueue::list_jobs` takes a `light` flag to skip heavy payload fields for dashboard lists
>>>>>>> 6e29489 (feat: optimize queries and organize drivers (#74))

## 0.0.24 - 2026-08-14

- Baseline changelog. Releases up to and including `0.0.24` are recorded only in
  the git tags (`v0.0.*`) and their GitHub Releases; per-crate history starts here.
