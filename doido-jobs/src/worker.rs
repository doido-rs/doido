use crate::queue::{JobPayload, JobQueue, Reserved};
use chrono::Utc;
use doido_core::Result;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Engine configuration. The engine is backend-agnostic: it only ever sees an
/// `Arc<dyn JobQueue>`.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Queue names in priority order (first = highest priority).
    pub queues: Vec<String>,
    /// Maximum jobs processed concurrently.
    pub concurrency: usize,
    /// Upper bound passed to `reserve` as the wait hint.
    pub poll_wait: Duration,
    /// How often expired leases are reclaimed.
    pub reclaim_interval: Duration,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            queues: vec!["default".to_string()],
            concurrency: 1,
            poll_wait: Duration::from_millis(500),
            reclaim_interval: Duration::from_secs(30),
        }
    }
}

/// Run one job's handler under its timeout, then finalize via ack / nack / dead_letter.
/// The retry-vs-dead-letter decision and backoff computation live here, so every
/// backend behaves identically.
async fn process<F, Fut>(queue: &Arc<dyn JobQueue>, reserved: Reserved, handler: &F) -> Result<()>
where
    F: Fn(JobPayload) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let job = reserved.job;
    let id = job.id.clone();
    let timeout = Duration::from_secs(job.timeout);

    let outcome = tokio::time::timeout(timeout, handler(job.clone())).await;

    match outcome {
        Ok(Ok(())) => queue.ack(&id).await?,
        Ok(Err(e)) => fail(queue, &job, &e.to_string()).await?,
        Err(_) => fail(queue, &job, "job timed out").await?,
    }
    Ok(())
}

/// Apply the failure policy: dead-letter if retries are exhausted, otherwise
/// re-enqueue at the backoff-derived `retry_at`.
async fn fail(queue: &Arc<dyn JobQueue>, job: &JobPayload, error: &str) -> Result<()> {
    if job.attempts >= job.max_retries {
        queue.dead_letter(&job.id, error).await?;
    } else {
        let delay = job.backoff.delay(job.attempts, job.backoff_base);
        let retry_at = Utc::now()
            + chrono::Duration::from_std(delay).unwrap_or_else(|_| chrono::Duration::zero());
        queue.nack(&job.id, Some(retry_at), error).await?;
    }
    Ok(())
}

/// Backend-agnostic worker engine: run loop, concurrency, timeout, backoff,
/// lease reclaim, and graceful shutdown.
pub struct WorkerEngine {
    queue: Arc<dyn JobQueue>,
    config: EngineConfig,
}

impl WorkerEngine {
    pub fn new(queue: Arc<dyn JobQueue>, config: EngineConfig) -> Self {
        Self { queue, config }
    }

    fn queue_refs(&self) -> Vec<&str> {
        self.config.queues.iter().map(|s| s.as_str()).collect()
    }

    /// Reserve and process at most one job. Returns `true` if a job was processed.
    /// The primitive used by tests and `TestQueue::drain`.
    pub async fn run_once<F, Fut>(&self, handler: &F) -> Result<bool>
    where
        F: Fn(JobPayload) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let queues = self.queue_refs();
        if let Some(reserved) = self.queue.reserve(&queues, self.config.poll_wait).await? {
            process(&self.queue, reserved, handler).await?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Run the engine until `shutdown` resolves, then drain in-flight jobs.
    pub async fn run<F, Fut>(&self, handler: F, shutdown: impl Future<Output = ()>) -> Result<()>
    where
        F: Fn(JobPayload) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));
        let reclaimer = self.spawn_reclaimer();
        tokio::pin!(shutdown);

        loop {
            // Acquire a slot before reserving so we never hold a lease we can't run.
            let permit = match Arc::clone(&semaphore).acquire_owned().await {
                Ok(p) => p,
                Err(_) => break,
            };

            let queues = self.queue_refs();
            let reserved = tokio::select! {
                biased;
                _ = &mut shutdown => { drop(permit); break; }
                r = self.queue.reserve(&queues, self.config.poll_wait) => r,
            };

            match reserved {
                Ok(Some(reserved)) => {
                    let queue = Arc::clone(&self.queue);
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(e) = process(&queue, reserved, &handler).await {
                            doido_core::tracing::error!("worker process error: {e}");
                        }
                    });
                }
                Ok(None) => drop(permit),
                Err(e) => {
                    doido_core::tracing::error!("worker reserve error: {e}");
                    drop(permit);
                }
            }
        }

        reclaimer.abort();
        // Drain: wait for all permits to be returned (in-flight jobs to finish).
        let _ = semaphore.acquire_many(self.config.concurrency as u32).await;
        Ok(())
    }

    fn spawn_reclaimer(&self) -> tokio::task::JoinHandle<()> {
        let queue = Arc::clone(&self.queue);
        let queues: Vec<String> = self.config.queues.clone();
        let interval = self.config.reclaim_interval;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let refs: Vec<&str> = queues.iter().map(|s| s.as_str()).collect();
                if let Err(e) = queue.reclaim_expired(&refs).await {
                    doido_core::tracing::error!("reclaim_expired error: {e}");
                }
            }
        })
    }
}

/// Single-queue convenience wrapper over `WorkerEngine` (back-compat).
pub struct Worker {
    engine: WorkerEngine,
}

impl Worker {
    pub fn new(queue: Arc<dyn JobQueue>, queue_name: impl Into<String>) -> Self {
        let config = EngineConfig {
            queues: vec![queue_name.into()],
            ..EngineConfig::default()
        };
        Self {
            engine: WorkerEngine::new(queue, config),
        }
    }

    /// Reserve and process at most one job.
    pub async fn run_once<F, Fut>(&self, performer: F) -> Result<()>
    where
        F: Fn(JobPayload) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        self.engine.run_once(&performer).await?;
        Ok(())
    }
}
