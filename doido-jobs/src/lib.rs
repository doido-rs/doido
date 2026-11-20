pub mod batch;
pub mod callbacks;
#[cfg(feature = "cli")]
pub mod commands;
pub mod config;
pub mod context;
pub mod cron;
pub mod drivers;
pub mod initializer;
pub mod inspect;
pub mod perform;
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
pub use config::{build_queue, default_sync_queue, Backend, JobsConfig, WebUiConfig};
pub use context::{job_context_with_queue, JobContext, JobQueueHandle};
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
pub use perform::{enqueue_jobs, enqueue_jobs_at, perform_jobs_now};
pub use queue::{BackoffStrategy, JobId, JobPayload, JobQueue, JobStatus, Reserved};
#[cfg(feature = "jobs-redis")]
pub use redis::RedisQueue;
pub use registry::{
    build_job_payload, build_job_payloads, parse_job_payload_json, parse_job_payloads_json,
    suggest_job_names, HandlerFn, HandlerFuture, JobDefaults, JobRegistration, JobRegistry,
};
pub use retry::{Decision, RetryPolicy};
#[cfg(feature = "webui")]
pub use webui::{build_jobs_webui_router, build_router, build_unprotected_jobs_webui_router};
pub use worker::{perform_job_now, EngineConfig, Worker, WorkerEngine};

pub use initializer::{default_initializer, on_boot, reset_for_test};

/// Retains job function symbols so the linker does not dead-code-strip their
/// `inventory::submit!` job registrations when compiling libraries.
#[macro_export]
macro_rules! link_jobs {
    ($($job:path),* $(,)?) => {
        $crate::link_jobs_internal(&[
            $(
                $job as *const (),
            )*
        ]);
    };
}

/// Alias for [`link_jobs!`].
#[macro_export]
macro_rules! retain_jobs {
    ($($args:tt)*) => {
        $crate::link_jobs!($($args)*);
    };
}

#[doc(hidden)]
pub fn link_jobs_internal(ptrs: &[*const ()]) {
    std::hint::black_box(ptrs);
}
