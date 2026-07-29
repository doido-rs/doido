pub mod batch;
pub mod callbacks;
pub mod config;
pub mod context;
#[cfg(feature = "jobs-db")]
pub mod db;
pub mod memory;
pub mod queue;
#[cfg(feature = "jobs-redis")]
pub mod redis;
pub mod retry;
pub mod worker;

// Re-exported so `#[job]`-generated fluent builders can name chrono types
// without the app depending on chrono directly.
pub use chrono;
pub use config::{build_queue, Backend, JobsConfig};
pub use context::JobContext;
#[cfg(feature = "jobs-db")]
pub use db::DbQueue;
pub use doido_jobs_macros::job;
pub use memory::{MemoryQueue, QueueStats};
pub use queue::{BackoffStrategy, JobId, JobPayload, JobQueue, JobStatus, Reserved};
#[cfg(feature = "jobs-redis")]
pub use redis::RedisQueue;
pub use retry::{Decision, RetryPolicy};
pub use worker::{EngineConfig, Worker, WorkerEngine};
