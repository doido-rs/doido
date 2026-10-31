//! The application context the [`WorkerEngine`](crate::WorkerEngine) carries and
//! hands to every job handler — the jobs analogue of the controller `Context`.

use crate::perform::{enqueue_jobs, enqueue_jobs_at};
use crate::queue::{JobId, JobPayload, JobQueue};
use chrono::{DateTime, Utc};
use doido_core::{anyhow::anyhow, Result};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "jobs-db")]
use doido_model::sea_orm::DatabaseConnection;

/// Configured queue backend (worker / CLI register this on [`JobContext`]).
#[derive(Clone)]
pub struct JobQueueHandle(pub Arc<dyn JobQueue>);

/// Shared application context passed to every job's handler.
///
/// A job reaches the application's database (behind `jobs-db`) plus any shared
/// state the worker registered — configuration, clients, caches — via a typed
/// extension store, rather than touching globals directly. The worker process
/// builds one at boot and hands it to
/// [`WorkerEngine::with_context`](crate::WorkerEngine::with_context).
#[derive(Clone, Default)]
pub struct JobContext {
    extensions: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl JobContext {
    /// Build the context handed to job handlers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a shared value, retrievable by type with [`get`](Self::get).
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) -> &mut Self {
        self.extensions.insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    /// Retrieve a previously registered value by type.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|value| value.clone().downcast::<T>().ok())
    }

    /// The application's database connection (global pool installed at boot).
    #[cfg(feature = "jobs-db")]
    pub fn db(&self) -> &'static DatabaseConnection {
        doido_model::pool::pool()
    }

    /// Attach the configured jobs backend so handlers can fan out with [`enqueue_jobs`](Self::enqueue_jobs).
    pub fn with_queue(mut self, queue: Arc<dyn JobQueue>) -> Self {
        self.insert(JobQueueHandle(queue));
        self
    }

    /// Queue installed by the worker or `doido jobs run` when a backend is available.
    pub fn queue(&self) -> Option<Arc<dyn JobQueue>> {
        self.get::<JobQueueHandle>().map(|h| Arc::clone(&h.0))
    }

    /// Bulk enqueue through the backend (`enqueue_many` — one SQL `INSERT` per chunk on db).
    pub async fn enqueue_jobs(
        &self,
        jobs: impl IntoIterator<Item = JobPayload>,
    ) -> Result<Vec<JobId>> {
        let q = self
            .queue()
            .ok_or_else(|| anyhow!("JobContext has no queue — register with with_queue()"))?;
        enqueue_jobs(q.as_ref(), jobs).await
    }

    /// Schedule many jobs at `at` in as few backend round-trips as possible.
    pub async fn enqueue_jobs_at(
        &self,
        jobs: impl IntoIterator<Item = JobPayload>,
        at: DateTime<Utc>,
    ) -> Result<Vec<JobId>> {
        let q = self
            .queue()
            .ok_or_else(|| anyhow!("JobContext has no queue — register with with_queue()"))?;
        enqueue_jobs_at(q.as_ref(), jobs, at).await
    }
}

/// Build the context the worker and `doido jobs run` hand to handlers (DB pool when installed).
pub fn job_context_from_runtime() -> JobContext {
    let mut job_ctx = JobContext::new();
    #[cfg(feature = "jobs-db")]
    if let Some(conn) = doido_model::pool::try_pool() {
        job_ctx.insert(conn.clone());
    }
    job_ctx
}

/// Runtime context plus the configured queue (worker / `--later` CLI).
pub fn job_context_with_queue(queue: Arc<dyn JobQueue>) -> JobContext {
    job_context_from_runtime().with_queue(queue)
}
