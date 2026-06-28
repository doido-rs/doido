# doido-jobs — Spec

Rails analogue: **Active Job**

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
```

A `build_queue(config) -> Arc<dyn JobQueue>` factory (mirrors `CacheRegistry`) reads `backend` and
constructs the matching implementation, so neither the engine nor the CLI knows which backend is live.

## Job Lifecycle

```
enqueue / enqueue_at → [queue]
        │ run_at reached
        ▼
   reserve (lease, visibility timeout) ──► perform()
        │                                    ├─ Ok       → ack (done)
        │                                    └─ Err/timeout → nack
        │                                          ├─ attempts < max_retries → re-enqueue at retry_at
        │                                          └─ attempts == max_retries → dead_letter
        ▼
   lease expires (worker died) → reclaim_expired → back to [queue]
```

## Dead Letter Queue

- Failed jobs written to a separate dead-letter store (same backend).
- Each entry stores: job payload, last error message, attempt count, failed_at timestamp.
- Inspectable and re-enqueueable via CLI:

```
doido jobs:failed             ← list dead letter jobs
doido jobs:retry <job_id>     ← re-enqueue a specific dead letter job
doido jobs:retry --all        ← re-enqueue all dead letter jobs
doido jobs:discard <job_id>   ← permanently remove from dead letter queue
```

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