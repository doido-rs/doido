//! Periodic cron scheduling: load tasks from config at worker boot and enqueue
//! registered `#[job]` handlers on a cron expression (UTC).

mod leader;
mod scheduler;

pub use leader::{build_cron_leader, CronLeaderLock, NoopCronLeader};
pub use scheduler::{boot, CompiledCronTask, CronScheduler};

use doido_core::{anyhow::anyhow, Environment, Result};
use serde::Deserialize;
use serde_json::Value;

/// One cron task as declared in `config/<env>.yml` under `jobs.cron.tasks`.
///
/// `schedule` uses six fields (UTC): `sec min hour day month weekday`
/// (e.g. `0 0 3 * * *` = daily at 03:00 UTC).
#[derive(Debug, Clone, Deserialize)]
pub struct CronTaskSettings {
    pub job_name: String,
    pub schedule: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub queue: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for CronTaskSettings {
    fn default() -> Self {
        Self {
            job_name: String::new(),
            schedule: String::new(),
            payload: Value::Null,
            queue: None,
            enabled: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// `jobs.cron` section from YAML.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CronSettings {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub tasks: Vec<CronTaskSettings>,
}

/// Runtime cron configuration after merging defaults and environment rules.
#[derive(Debug, Clone)]
pub struct CronConfig {
    pub enabled: bool,
    pub tasks: Vec<CronTaskSettings>,
}

impl CronSettings {
    /// Build runtime cron config. Cron is off in [`Environment::Test`] unless
    /// explicitly `enabled: true`. When `enabled` is omitted, cron runs if there
    /// is at least one task.
    pub fn into_config(self, env: Environment) -> CronConfig {
        let has_tasks = !self.tasks.is_empty();
        let mut enabled = self.enabled.unwrap_or(has_tasks);
        if env == Environment::Test && self.enabled != Some(true) {
            enabled = false;
        }
        CronConfig {
            enabled,
            tasks: self.tasks,
        }
    }
}

impl CronConfig {
    pub fn is_active(&self) -> bool {
        self.enabled && self.tasks.iter().any(|t| t.enabled)
    }

    /// Parse schedules and validate job names against the registry.
    pub fn compile(
        &self,
        registry: &crate::registry::JobRegistry,
    ) -> Result<Vec<CompiledCronTask>> {
        let mut out = Vec::new();
        for (index, task) in self.tasks.iter().enumerate() {
            if !task.enabled {
                continue;
            }
            let schedule = task.schedule.parse::<cron::Schedule>().map_err(|e| {
                anyhow!(
                    "jobs.cron.tasks[{index}] job `{}`: invalid cron schedule `{}`: {e}",
                    task.job_name,
                    task.schedule
                )
            })?;
            if registry.defaults_for(&task.job_name).is_none() {
                let known = registry.names();
                let hints = crate::registry::suggest_job_names(&known, &task.job_name, 5);
                let mut msg = format!("jobs.cron.tasks[{index}]: unknown job `{}`", task.job_name);
                if !hints.is_empty() {
                    msg.push_str("\ndid you mean:");
                    for h in &hints {
                        msg.push_str("\n  ");
                        msg.push_str(h);
                    }
                }
                return Err(anyhow!(msg));
            }
            out.push(CompiledCronTask {
                job_name: task.job_name.clone(),
                schedule,
                payload: task.payload.clone(),
                queue: task.queue.clone(),
            });
        }
        Ok(out)
    }
}
