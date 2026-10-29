use crate::cron::CronConfig;
use crate::cron::CronLeaderLock;
use crate::queue::JobQueue;
use crate::registry::{build_job_payload, JobRegistry};
use chrono::{DateTime, Utc};
use cron::Schedule;
use doido_core::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

/// A cron task with a parsed schedule, ready to run.
#[derive(Clone)]
pub struct CompiledCronTask {
    pub job_name: String,
    pub schedule: Schedule,
    pub payload: serde_json::Value,
    pub queue: Option<String>,
}

impl CompiledCronTask {
    /// Next fire strictly after `from` (UTC).
    pub fn next_run_after(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.schedule.after(&from).next()
    }

    pub fn slot_key(&self, fire_at: DateTime<Utc>) -> String {
        format!("doido:cron:{}:{}", self.job_name, fire_at.timestamp())
    }
}

/// Spawns one async loop per compiled task; returns handles to abort on shutdown.
pub struct CronScheduler;

impl CronScheduler {
    pub fn spawn(
        tasks: Vec<CompiledCronTask>,
        queue: Arc<dyn JobQueue>,
        registry: Arc<JobRegistry>,
        leader: Arc<dyn CronLeaderLock>,
    ) -> Vec<JoinHandle<()>> {
        tasks
            .into_iter()
            .map(|task| {
                let queue = Arc::clone(&queue);
                let registry = Arc::clone(&registry);
                let leader = Arc::clone(&leader);
                tokio::spawn(async move {
                    if let Err(e) = run_task_loop(task, queue, registry, leader).await {
                        doido_core::tracing::error!("cron task loop error: {e:#}");
                    }
                })
            })
            .collect()
    }

    /// Validate config, log next runs, compile tasks.
    pub fn prepare(cron: &CronConfig, registry: &JobRegistry) -> Result<Vec<CompiledCronTask>> {
        if !cron.is_active() {
            return Ok(Vec::new());
        }
        let tasks = cron.compile(registry)?;
        let now = Utc::now();
        for task in &tasks {
            let next = task
                .next_run_after(now)
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| "(none)".to_string());
            doido_core::tracing::info!(
                job_name = %task.job_name,
                next_run = %next,
                "cron task loaded"
            );
        }
        Ok(tasks)
    }
}

/// Enqueue one cron firing at `fire_at` when the leader lock is acquired.
pub(crate) async fn enqueue_if_due(
    task: &CompiledCronTask,
    queue: &Arc<dyn JobQueue>,
    registry: &JobRegistry,
    leader: &Arc<dyn CronLeaderLock>,
    fire_at: DateTime<Utc>,
) -> Result<bool> {
    let slot = task.slot_key(fire_at);
    if !leader.try_acquire(&slot).await? {
        doido_core::tracing::debug!(
            job_name = %task.job_name,
            slot = %slot,
            "cron tick skipped (another worker holds the lock)"
        );
        return Ok(false);
    }

    let job = build_job_payload(
        registry,
        &task.job_name,
        task.payload.clone(),
        task.queue.clone(),
        None,
        None,
    )?;
    match queue.enqueue(job).await {
        Ok(id) => {
            doido_core::tracing::info!(
                job_name = %task.job_name,
                job_id = %id,
                fire_at = %fire_at.to_rfc3339(),
                "cron enqueued job"
            );
            Ok(true)
        }
        Err(e) => {
            doido_core::tracing::error!(
                job_name = %task.job_name,
                error = %e,
                "cron enqueue failed"
            );
            Err(e)
        }
    }
}

async fn run_task_loop(
    task: CompiledCronTask,
    queue: Arc<dyn JobQueue>,
    registry: Arc<JobRegistry>,
    leader: Arc<dyn CronLeaderLock>,
) -> Result<()> {
    loop {
        let now = Utc::now();
        let from = now - chrono::Duration::seconds(1);
        let Some(fire_at) = task.next_run_after(from) else {
            tokio::time::sleep(Duration::from_secs(60)).await;
            continue;
        };

        if fire_at > now {
            let sleep_until = tokio::time::Instant::now() + duration_until(fire_at, now);
            tokio::time::sleep_until(sleep_until).await;
        }
        let _ = enqueue_if_due(&task, &queue, &registry, &leader, fire_at).await?;
    }
}

/// Validate cron config and spawn scheduler loops (used by the worker CLI).
pub async fn boot(
    cron: &CronConfig,
    config: &crate::config::JobsConfig,
    queue: Arc<dyn JobQueue>,
    registry: Arc<JobRegistry>,
) -> doido_core::Result<Vec<JoinHandle<()>>> {
    if !cron.is_active() {
        return Ok(Vec::new());
    }
    let tasks = CronScheduler::prepare(cron, &registry)?;
    if tasks.is_empty() {
        return Ok(Vec::new());
    }
    let leader = crate::cron::build_cron_leader(config).await?;
    Ok(CronScheduler::spawn(tasks, queue, registry, leader))
}

fn duration_until(fire_at: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
    fire_at
        .signed_duration_since(now)
        .to_std()
        .unwrap_or(Duration::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cron::{CronConfig, CronTaskSettings, NoopCronLeader};
    use crate::memory::MemoryQueue;
    use chrono::{Datelike, TimeZone, Timelike};
    use std::sync::Arc;

    struct DenyLeader;

    #[async_trait::async_trait]
    impl CronLeaderLock for DenyLeader {
        async fn try_acquire(&self, _slot_key: &str) -> doido_core::Result<bool> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn enqueue_if_due_enqueues_when_leader_accepts() {
        let schedule: Schedule = "0 0 3 * * *".parse().unwrap();
        let task = CompiledCronTask {
            job_name: "tick".into(),
            schedule,
            payload: serde_json::json!({}),
            queue: None,
        };
        let mem = Arc::new(MemoryQueue::new());
        let queue: Arc<dyn JobQueue> = mem.clone();
        let mut registry = JobRegistry::new();
        registry.register("tick", |_job, _ctx| Box::pin(async { Ok(()) }));
        let leader: Arc<dyn CronLeaderLock> = Arc::new(NoopCronLeader);
        let fire_at = Utc.with_ymd_and_hms(2026, 4, 15, 3, 0, 0).unwrap();
        assert!(enqueue_if_due(&task, &queue, &registry, &leader, fire_at)
            .await
            .unwrap());
        assert!(queue
            .reserve(&["default"], Duration::from_millis(50))
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn enqueue_if_due_skips_when_leader_denies() {
        let schedule: Schedule = "0 0 3 * * *".parse().unwrap();
        let task = CompiledCronTask {
            job_name: "tick".into(),
            schedule,
            payload: serde_json::json!({}),
            queue: None,
        };
        let queue: Arc<dyn JobQueue> = Arc::new(MemoryQueue::new());
        let mut registry = JobRegistry::new();
        registry.register("tick", |_job, _ctx| Box::pin(async { Ok(()) }));
        let leader: Arc<dyn CronLeaderLock> = Arc::new(DenyLeader);
        let fire_at = Utc.with_ymd_and_hms(2026, 4, 15, 3, 0, 0).unwrap();
        assert!(!enqueue_if_due(&task, &queue, &registry, &leader, fire_at)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn boot_spawns_handles_for_active_cron() {
        let cron = CronConfig {
            enabled: true,
            tasks: vec![CronTaskSettings {
                job_name: "tick".into(),
                schedule: "0 0 3 * * *".into(),
                ..Default::default()
            }],
        };
        let config = crate::config::JobsConfig::default();
        let queue = Arc::new(MemoryQueue::new());
        let mut registry = JobRegistry::new();
        registry.register("tick", |_job, _ctx| Box::pin(async { Ok(()) }));
        let handles = boot(&cron, &config, queue, Arc::new(registry))
            .await
            .unwrap();
        assert_eq!(handles.len(), 1);
        for h in handles {
            h.abort();
        }
    }

    #[test]
    fn next_run_after_finds_following_minute() {
        let schedule: Schedule = "0 0 3 * * *".parse().unwrap();
        let task = CompiledCronTask {
            job_name: "x".into(),
            schedule,
            payload: serde_json::json!({}),
            queue: None,
        };
        let from = Utc.with_ymd_and_hms(2026, 4, 15, 2, 0, 0).unwrap();
        let next = task.next_run_after(from).unwrap();
        assert_eq!(next.hour(), 3);
        assert_eq!(next.day(), 15);
    }
}
