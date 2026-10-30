# doido-jobs — Spec

Rails analogue: **Active Job**

> **Status (2026-10-20): done.** `JobQueue` trait, worker engine, leasing, exponential
> backoff, dead-letter, memory/db/redis backends, the `#[job]` macro (including the fluent
> `<Name>Job` builder), link-time handler registry, and CLI: `doido jobs list`, `doido jobs
> run` (`perform_now` / `--later`), plus dead-letter admin (`failed` / `retry` / `discard`).
> See [ARCHITECTURE.md](ARCHITECTURE.md) and [doido-jobs/README.md](../doido-jobs/README.md).

## Decisions (resolved in interview)

- **Queue backends: pluggable** — in-memory, database (sea-orm), and Redis selectable via config.
- **The worker engine is fully decoupled from the queue provider** — it depends only on the
  `JobQueue` trait and an `Arc<dyn JobQueue>` handed to it by a config-driven builder. The same
  engine drives every backend.
- **Reliable delivery via leasing** — `reserve` leases a job (it becomes invisible for a
  visibility-timeout window) instead of destructively popping it. A crashed worker's in-flight
  jobs are reclaimed and re-run; jobs are not lost mid-`perform`.
- **Retry: automatic, exponential backoff by default**, configured per-job on the struct. The
  engine computes the next `retry_at` and hands it to the backend — every backend retries
  identically.
- **Exhausted jobs written to a dead letter queue** — inspectable and re-enqueueable via CLI.

## Job Definition

```rust
#[job(queue = "default", max_retries = 5, backoff = "exponential")]
struct SendWelcomeEmail {
    pub user_id: i64,
}

impl SendWelcomeEmail {
    async fn perform(&self, ctx: &JobContext) -> Result<()> {
        let user = User::find_by_id(self.user_id).one(&ctx.db).await?;
        UserMailer::welcome(&user).deliver_now().await?;
        Ok(())
    }
}
```

## `#[job(...)]` Macro Attributes

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `queue` | `&str` | `"default"` | Queue name |
| `max_retries` | `u32` | `3` | Max attempts before dead letter |
| `backoff` | `&str` | `"exponential"` | `"exponential"` \| `"linear"` \| `"none"` |
| `backoff_base` | `u64` (seconds) | `5` | Base delay for backoff calculation |
| `timeout` | `u64` (seconds) | `30` | Max seconds a single attempt may run |
| `priority` | `i32` | `0` | Higher = processed first |

The macro stamps these onto every `JobPayload` it enqueues, so the engine can read the retry,
timeout, and priority policy off the job itself without any per-backend configuration.

Backoff delay formula (`attempt` is 1-based, the attempt that just failed):

| Strategy | Delay |
|----------|-------|
| `exponential` | `backoff_base * 2^(attempt - 1)` |
| `linear` | `backoff_base * attempt` |
| `none` | `0` (retry immediately) |

Exponential with `backoff_base = 5`: attempt 1 → 5s, 2 → 10s, 3 → 20s, 4 → 40s, 5 → 80s.

## `JobPayload`

The unit of work moved through the queue and persisted by every backend.

```rust
pub struct JobPayload {
    pub id: String,                 // uuid v4
    pub queue: String,
    pub payload: serde_json::Value, // serialized job struct
    pub attempts: u32,
    pub max_retries: u32,
    pub status: JobStatus,          // Pending | Running | Done | Failed | Dead
    pub error: Option<String>,
    pub priority: i32,              // higher first
    pub run_at: DateTime<Utc>,     // not eligible to reserve until this time
    pub backoff: BackoffStrategy,  // Exponential | Linear | None
    pub backoff_base: u64,         // seconds
    pub timeout: u64,              // seconds, per attempt
}
```

New fields carry serde defaults so older serialized payloads still deserialize.

## `JobQueue` Trait (pluggable, lease-based)

The engine talks to backends *only* through this trait.

```rust
pub struct Reserved {
    pub job: JobPayload,
    pub lease_until: DateTime<Utc>, // visibility timeout
}

#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Enqueue for immediate eligibility (run_at = now).
    async fn enqueue(&self, job: JobPayload) -> Result<JobId>;

    /// Enqueue, eligible no earlier than `at` (delays + scheduled retries).
    async fn enqueue_at(&self, job: JobPayload, at: DateTime<Utc>) -> Result<JobId>;

    /// Atomically lease the next eligible job across `queues`, honoring priority and
    /// `run_at`. `wait` lets blocking backends (Redis BRPOP, memory Notify) park instead
    /// of busy-spinning; polling backends (DB) may sleep internally up to `wait`.
    async fn reserve(&self, queues: &[&str], wait: Duration) -> Result<Option<Reserved>>;

    /// Job succeeded — remove it.
    async fn ack(&self, id: &str) -> Result<()>;

    /// Job failed but has retries left — make it eligible again at `retry_at`
    /// (None = immediately).
    async fn nack(&self, id: &str, retry_at: Option<DateTime<Utc>>, error: &str) -> Result<()>;

    /// Return leased-but-expired jobs to their queue (crash recovery). Returns count reclaimed.
    async fn reclaim_expired(&self, queues: &[&str]) -> Result<u64>;

    /// Move an exhausted job to the dead letter store.
    async fn dead_letter(&self, id: &str, reason: &str) -> Result<()>;

    /// Inspect the dead letter store for a queue.
    async fn dead_jobs(&self, queue: &str) -> Result<Vec<JobPayload>>;
}
```

The retry-vs-dead-letter decision lives in the **engine**, not the backend: on failure the engine
checks `attempts >= max_retries` and calls either `dead_letter` or `nack(retry_at)`.

## Worker Engine

`WorkerEngine` owns all execution policy and is backend-agnostic.

```rust
pub struct EngineConfig {
    pub queues: Vec<String>,        // priority order across queues
    pub concurrency: usize,         // max jobs in flight
    pub poll_wait: Duration,        // reserve() wait hint
    pub reclaim_interval: Duration, // how often to reclaim expired leases
}

let engine = WorkerEngine::new(queue, config);
engine.run(handler, shutdown_signal).await?; // loops until shutdown
engine.run_once(&handler).await?;             // process at most one job (tests/drain)
```

Responsibilities:

- Loop calling `reserve(queues, poll_wait)`; dispatch each leased job to the `handler`.
- Bound in-flight work with a `tokio::Semaphore` sized to `concurrency`.
- Enforce per-attempt `timeout` via `tokio::time::timeout`; a timeout counts as a failure.
- On failure compute `retry_at` from the job's `BackoffStrategy` and either `nack` or `dead_letter`.
- A background tick calls `reclaim_expired` every `reclaim_interval`.
- **Graceful shutdown**: stop reserving, let in-flight jobs finish, then return.

`Worker` is a thin single-queue convenience wrapper over the engine (back-compat for
`Worker::new(queue, name)` + `run_once`).

## Built-in Backends

| Backend | Feature flag | Reserve mechanism | Scheduled jobs | Crash recovery |
|---------|--------------|-------------------|----------------|----------------|
| `MemoryQueue` | always | `VecDeque` pop + `tokio::Notify` | `run_at` check + in-memory delayed set | `running` map with `lease_until`; reclaim past-due |
| `DbQueue` | `jobs-db` | `SELECT … FOR UPDATE SKIP LOCKED` | `WHERE run_at <= now()` | `locked_at` column + visibility timeout |
| `RedisQueue` | `jobs-redis` | `RPOPLPUSH ready → processing` | `ZSET` scored by epoch, poller promotes due jobs | scan `processing` list for stale leases |

Selected via config:

```toml
[jobs]
backend = "db"          # "memory" | "db" | "redis"
queues = ["critical", "default", "mailers"]   # priority order, highest first
concurrency = 5
poll_wait_ms = 1000
reclaim_interval_secs = 30

[jobs.redis]
url = "${REDIS_URL}"
namespace = "doido:jobs"

[jobs.db]
visibility_timeout_secs = 300
sql_logging = false          # optional: dedicated jobs DB connection without SQL trace noise
```

When `jobs.db.sql_logging` is `false`, the worker opens a separate database connection
with sea-orm statement logging disabled for queue operations (`SELECT`/`UPDATE` on
`doido_jobs`), while job handlers still use the global pool (and `logger.sql`) for
application queries.

Failed attempts persist the error on the row: in the JSON `data` payload and in the
`last_error` column (visible in SQL clients and after `nack` / `dead_letter`).

A `build_queue(config) -> Arc<dyn JobQueue>` factory (mirrors `CacheRegistry`) reads `backend` and
constructs the matching implementation, so neither the engine nor the CLI knows which backend is live.

## Job Lifecycle

`attempts` increments on each `reserve`; `max_retries` is the maximum number of
**executions** (including the first run), not “extra retries after the first failure”.

```
enqueue / enqueue_at → Pending (run_at)
        │ run_at reached
        ▼
   reserve (lease) ──► Running ──► perform()
        │                         ├─ Ok → ack (row removed / done)
        │                         └─ Err / timeout / panic
        │                               ├─ attempts < max_retries → Failed (run_at = retry_at, backoff)
        │                               └─ attempts >= max_retries → Dead (dead-letter store)
        ▼
   Failed ──► reserve when run_at due ──► Running (retry)
        ▼
   lease expires (worker died) → reclaim_expired → Pending/Failed with run_at unchanged
```

After a retryable failure the engine calls `nack`: the job stays in the same logical
queue with status **`Failed`**, `run_at` set from the job’s `BackoffStrategy`, and
`last_error` populated (DB backend). When `run_at` is due, `reserve` claims it like
any other eligible work.

## Dead Letter Queue

- Exhausted jobs (`Dead` status) live in the dead-letter store (same backend).
- Each entry stores: job payload, last error message, attempt count, failed_at timestamp.
- Inspectable and re-enqueueable via CLI (see also **Running jobs from the CLI** below):

```
doido jobs failed             ← list dead-letter jobs (tracing output)
doido jobs retry              ← re-enqueue every dead job (fresh id, attempts reset, job_name kept), then clear the dead store
doido jobs discard            ← permanently remove all dead jobs
```

## Web UI (dashboard)

When `jobs.webui.enabled` is true, the HTTP server mounts a Mission Control–style
dashboard (default **`/doido/jobs`**) on boot. It uses the same configured queue
backend as the worker (`memory` / `db` / `redis`).

Tabs:

| Tab | Shows |
|-----|--------|
| **Queues** | Per-queue counts (ready, scheduled, running, retry wait, failed-ready, dead) |
| **In queue** | Pending jobs ready now plus failed jobs ready to run again |
| **Executing** | Leased / running jobs |
| **Retry** | Failed jobs still waiting for backoff (`run_at` in the future) |
| **Dead** | Dead-letter jobs with per-job and bulk **retry** / **discard** actions |

Job and queue tables paginate with opaque **`page_token`** / **`queue_page_token`**
query params (cursor-based on job sort key: priority, run-at, id — not offset/limit).

**Performance:** the **Queues** tab alone loads per-queue counts (`queue_counts`).
Job tabs list jobs without calling `queue_counts`. Payload JSON is omitted by default;
add **`include_payload=1`** to show the payload column (or use **Show payload** on a row).
Optional **`stats_ttl_secs`** caches queue counts in-process on the Queues tab.

Queue backends live under `doido-jobs` **`drivers/`** (`memory`, `db`, `redis`); the
public imports (`doido_jobs::db::DbQueue`, etc.) are unchanged.

```yaml
jobs:
  type: db
  queues: [default, mailers]
  webui:
    enabled: true
    mount_path: /doido/jobs
    stats_ttl_secs: 30
    username: admin
    password: "{{ get_env(name=JOBS_WEBUI_PASSWORD, default=change-me) }}"
```

- **`webui.enabled`** defaults to `false` (no routes).
- **`webui.stats_ttl_secs`** — optional in-process cache TTL for Queues-tab stats (omit to refresh every request).
- In **production**, enabling the dashboard without **`username`** and **`password`**
  logs a warning and **does not mount** routes.
- When credentials are set, the dashboard is protected with **HTTP Basic Auth**.
- Build `doido-jobs` with the **`webui`** feature (enabled on the `doido` metacrate).

## Periodic cron (worker)

When `cargo doido worker` starts (long-lived mode, not `--once`), it reads
`jobs.cron` from `config/<env>.yml`, validates each task’s cron expression and
registered `job_name`, and enqueues a `JobPayload` on each tick. Schedules are
**reloaded only at worker boot** (no hot-reload). All times are **UTC**.

```yaml
jobs:
  type: db
  cron:
    enabled: true          # default: true when tasks are present
    tasks:
      - job_name: sync_catalog_daily
        schedule: "0 0 3 * * *"   # sec min hour dom month dow
        payload: {"skip_reschedule": true}
        queue: catalog            # optional; default from #[job]
        enabled: true
```

Expressions use **six fields** (`sec min hour day month weekday`), as parsed by
the `cron` crate. Cron is **off in `test`** unless `cron.enabled: true` is set
explicitly.

**Multiple workers:** each tick acquires a short-lived leader lock before enqueue
(Redis `SET NX` when `jobs.type: redis`; Postgres/MySQL advisory locks when
`jobs.type: db` and the app DB is not SQLite). Memory/SQLite dev setups log a
warning and behave like a single worker.

`cargo doido worker --once` does **not** start the cron scheduler (drain-only).

## Running jobs from the CLI

Rails analogue: **`perform_now`** vs **`perform_later`**.

| Rails | Doido |
|-------|-------|
| `perform_later` | `cargo doido jobs run <name> --later` |
| `perform_now` | `cargo doido jobs run <name>` |
| Job processor | `cargo doido worker` / `cargo doido worker --once` |

Handlers must be linked into the app binary (same as the worker). `jobs run` builds
`JobContext` and, for the `db` backend, installs the global pool via
`build_configured_queue` before dispatch.

```bash
cargo doido jobs list
cargo doido jobs run sync_catalog --payload '{"skip_reschedule":true}'
cargo doido jobs run import_anexo_a --queue catalog
cargo doido jobs run sync_lote --later   # enqueue; drain with worker --once
```

Flags: `--payload` (JSON, default `{}`), `--queue`, `--timeout`, `--max-retries` (for
`--later`). Defaults for queue/timeout/retries come from `#[job(...)]` metadata stamped
into `JobRegistration` at compile time. In-process runs use `perform_job_now` (timeout +
registry dispatch, no queue `ack`/`nack`). Exit code `0` on success, non-zero on unknown
job name, bad JSON, handler error, or timeout.

## Enqueue API

```rust
// Enqueue immediately
SendWelcomeEmail { user_id: 42 }.enqueue().await?;

// Enqueue with delay
SendWelcomeEmail { user_id: 42 }.enqueue_in(Duration::from_secs(60)).await?;

// Enqueue at specific time
SendWelcomeEmail { user_id: 42 }.enqueue_at(scheduled_at).await?;

// Enqueue to specific queue
SendWelcomeEmail { user_id: 42 }.on_queue("critical").enqueue().await?;
```

## Test Helpers

```rust
use doido_jobs::testing::JobQueue as TestQueue;

// Assert a job was enqueued
TestQueue::assert_enqueued::<SendWelcomeEmail>(|job| job.user_id == 42);

// Drain and execute all enqueued jobs
TestQueue::drain(&ctx).await?;

// Assert dead letter queue
TestQueue::assert_dead_lettered::<SendWelcomeEmail>(1);
```

## Known Requirements

- `#[job(...)]` macro implements job serialization to JSON and stamps retry/timeout/priority policy.
- Job structs must be `Serialize + Deserialize` (serde).
- `JobContext` carries `db: DatabaseConnection` and app config.
- Worker process started via `doido worker` or `doido server` (embedded worker mode); the CLI builds
  the backend from config and runs `WorkerEngine`.
- In test env, `MemoryQueue` always used; jobs do not auto-execute unless `drain()` called.
- Dead letter entries never auto-deleted; require explicit CLI action.

## TDD Surface

- Test `#[job]` macro generates correct serialization and stamps backoff/timeout/priority.
- Test `enqueue()` makes a job reservable; `enqueue_at()` defers it until `run_at`.
- Test `reserve()` leases a job (status Running, attempts incremented) and respects priority order.
- Test `ack()` removes a reserved job; a second `reserve()` returns `None`.
- Test failed `perform()` → `nack` increments attempts and re-enqueues at `retry_at`.
- Test exponential/linear/none backoff delays are calculated correctly.
- Test job moved to dead letter after `max_retries` exhausted.
- Test `reclaim_expired()` returns an expired lease to the queue and not a fresh one.
- Test engine honors `concurrency` and processes jobs across multiple queues.
- Test engine `timeout` turns a hung attempt into a retry.
- Test graceful shutdown drains in-flight jobs.
- Backend parity: the same suite runs against memory, db (sqlite in-memory), and redis (gated on `REDIS_URL`).
- Integration: controller enqueues job → drain in test → side effect observed.
```