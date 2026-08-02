+++
title = "Jobs"
description = "Background jobs with the #[job] macro, pluggable queues, workers, retries with backoff, and a dead-letter queue."
weight = 8
aliases = ['/docs/guides/jobs/']

+++

> **Design spec:** [`docs/09-jobs.md`](https://github.com/doido-rs/doido/blob/master/docs/09-jobs.md).
> This guide documents the API as implemented in `doido-jobs`.

**Rails analogue: Active Job.** Background jobs are plain async functions annotated with
`#[job]`; they enqueue onto a pluggable queue (in-memory, database, or Redis) and are run
by a worker that handles concurrency, per-job timeouts, retries with backoff, reliable
leasing, and a dead-letter queue.

## At a glance

```rust
use doido::jobs::{job, JobQueue, JobPayload, MemoryQueue, Worker};
use std::sync::Arc;
```

## Defining a job

`#[job]` stamps the queue, retry policy, backoff, timeout, and priority onto the job and
generates a `<name>_enqueue` helper. The job's payload is its last typed parameter
(defaulting to `serde_json::Value`), which must be `Serialize`.

```rust
use doido::jobs::job;

#[job(queue = "emails", max_retries = 5, backoff = "exponential", backoff_base = 5, timeout = 30, priority = 7)]
async fn send_welcome(user_id: i64) -> doido::Result<()> {
    // …deliver the welcome email…
    Ok(())
}
```

## Enqueuing

The generated `<name>_enqueue(queue, payload)` serializes the payload and enqueues it,
returning a `JobId`.

```rust
let queue = Arc::new(MemoryQueue::new());
let id = send_welcome_enqueue(queue.as_ref(), 42).await?; // immediate
```

For **delayed** or **scheduled** delivery, build a `JobPayload` and use the queue's
fluent setters (`with_wait`, `with_run_at`, `with_priority`, …):

```rust
use doido::jobs::JobPayload;

let job = JobPayload::new("emails", serde_json::json!({ "user_id": 42 }), 5)
    .with_wait(300)          // run in 5 minutes
    .with_priority(9);
queue.enqueue(job).await?;

// …or an absolute time:
queue.enqueue_at(JobPayload::new("emails", payload, 5), when).await?;
```

## Choosing a backend

`JobQueue` is the pluggable trait; pick a backend with `build_queue(&JobsConfig)` (reads
the `jobs` config section) or construct one directly. `MemoryQueue` is always available;
`DbQueue` (feature `jobs-db`) and `RedisQueue` (feature `jobs-redis`) add durability.

```rust
use doido::jobs::{build_queue, JobsConfig};

let queue = build_queue(&JobsConfig::default()).await?; // Arc<dyn JobQueue>
```

```yaml
# config/production.yml
jobs:
  backend: redis            # memory | db | redis
  queues: [critical, default, emails]
  concurrency: 8
  redis_url: redis://127.0.0.1:6379
```

## Running the worker

The worker reserves jobs (leasing them so a crashed worker's jobs are reclaimed) and runs
your handler. Use `WorkerEngine` for multi-queue/concurrency control, or `Worker` for a
single queue; `run_once` drains once (great for tests), `run` loops until shutdown. The
`cargo doido worker` command runs it as a process.

```rust
use doido::jobs::{Worker, WorkerEngine, EngineConfig};

// Single queue:
Worker::new(queue.clone(), "emails")
    .run_once(|job, _ctx| async move {
        // dispatch on job.queue / job.payload and perform the work
        Ok(())
    })
    .await?;

// Multi-queue engine with concurrency, running until Ctrl-C:
let engine = WorkerEngine::new(queue.clone(), EngineConfig::default());
engine.run(handler, tokio::signal::ctrl_c().map(|_| ())).await?;
```

```bash
cargo doido worker            # process jobs continuously
cargo doido worker --once     # drain the queue and exit
```

## Retries & backoff

Failing jobs are retried up to `max_retries` with the configured `BackoffStrategy`
(`Exponential`, `Linear`, or `None`) and `backoff_base` seconds — all set on `#[job]`. The
worker `nack`s a failure with the computed `retry_at`, and `reclaim_expired` replays leases
lost to a crash.

```rust
#[job(max_retries = 5, backoff = "exponential", backoff_base = 10)]
async fn charge_card(order_id: i64) -> doido::Result<()> { Ok(()) }
```

## Dead-letter queue

Jobs that exhaust their retries are moved to a dead-letter store, inspectable and
re-runnable via the CLI.

```bash
cargo doido jobs:failed            # list dead-lettered jobs
cargo doido jobs:retry <job_id>    # requeue one
cargo doido jobs:discard <job_id>  # drop one
```

## Priority & scheduling

`priority` (higher runs sooner) is set on `#[job]` or per payload with `with_priority`;
`with_wait(secs)` / `with_run_at(datetime)` schedule future execution.

## Job context

A `JobContext` carries shared state (and a DB connection with `jobs-db`) into handlers; a
`WorkerEngine` can be built `with_context` to inject application state.

```rust
use doido::jobs::{JobContext, WorkerEngine, EngineConfig};

let engine = WorkerEngine::with_context(queue, EngineConfig::default(), JobContext::new());
```

## Testing

`MemoryQueue` plus `reserve` / `run_once` make jobs deterministic in tests — enqueue, then
drain and assert.

```rust
let queue = Arc::new(MemoryQueue::new());
send_welcome_enqueue(queue.as_ref(), 1).await?;
let reserved = queue.reserve(&["emails"], std::time::Duration::from_millis(50)).await?;
assert!(reserved.is_some());
```

## Spec vs. implementation

> Enqueue is a **generated free function** (`send_welcome_enqueue(queue, payload)`), not a
> method on the job struct; the spec's `Job.perform_later` shape maps onto this helper plus
> `JobPayload`'s fluent setters.

## See also

- [Mailer](@/docs/reference/mailer.md) — `deliver_later` enqueues onto the `mailers` queue.
- [Cache](@/docs/reference/cache.md) & [Models](@/docs/reference/models.md) — Redis/DB backends.
- [Generators & CLI](@/docs/reference/generators.md) — `cargo doido generate job` and `cargo doido worker`.
