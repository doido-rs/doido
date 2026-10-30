pub mod batch;
pub mod callbacks;
#[cfg(feature = "cli")]
pub mod commands;
pub mod config;
pub mod context;
pub mod cron;
pub mod drivers;
pub mod inspect;
pub mod queue;
pub mod registry;
pub mod retry;
#[cfg(feature = "webui")]
pub mod webui;
pub mod worker;

// Re-exported so `#[job]`-generated fluent builders can name chrono types
// without the app depending on chrono directly.
pub use chrono;
// Re-exported so `#[job]`-generated code can call `doido_jobs::inventory::submit!`
// without the app depending on inventory directly.
pub use config::{build_queue, Backend, JobsConfig, WebUiConfig};
pub use context::JobContext;
#[cfg(feature = "jobs-db")]
pub use db::DbQueue;
pub use doido_jobs_macros::job;
#[cfg(feature = "jobs-db")]
#[doc(inline)]
pub use drivers::db;
#[cfg(feature = "jobs-db")]
pub use drivers::db::entity;
#[doc(inline)]
pub use drivers::memory;
#[cfg(feature = "jobs-redis")]
#[doc(inline)]
pub use drivers::redis;
pub use inspect::{classify_stored, JobBucket, QueueCounts};
pub use inventory;
pub use memory::{MemoryQueue, QueueStats};
pub use queue::{BackoffStrategy, JobId, JobPayload, JobQueue, JobStatus, Reserved};
#[cfg(feature = "jobs-redis")]
pub use redis::RedisQueue;
pub use registry::{
    build_job_payload, suggest_job_names, HandlerFn, HandlerFuture, JobDefaults, JobRegistration,
    JobRegistry,
};
pub use retry::{Decision, RetryPolicy};
pub use worker::{perform_job_now, EngineConfig, Worker, WorkerEngine};
