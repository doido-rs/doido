**Unstable**

## What's changed in 0.0.32-test3

### doido-jobs

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
