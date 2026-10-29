use doido_jobs::config::JobsFileConfig;
use doido_jobs::cron::{boot, CronConfig, CronScheduler, CronTaskSettings};
use doido_jobs::memory::MemoryQueue;
use doido_jobs::registry::JobRegistry;
use std::sync::Arc;

#[test]
fn prepare_returns_empty_when_cron_inactive() {
    let cron = CronConfig {
        enabled: false,
        tasks: vec![CronTaskSettings {
            job_name: "x".into(),
            schedule: "0 0 * * * *".into(),
            ..Default::default()
        }],
    };
    let registry = JobRegistry::new();
    assert!(CronScheduler::prepare(&cron, &registry).unwrap().is_empty());
}

#[tokio::test]
async fn boot_returns_no_handles_when_cron_disabled() {
    let cron = CronConfig {
        enabled: false,
        tasks: vec![],
    };
    let queue = Arc::new(MemoryQueue::new());
    let registry = Arc::new(JobRegistry::new());
    let handles = boot(
        &cron,
        &doido_jobs::config::JobsConfig::default(),
        queue,
        registry,
    )
    .await
    .unwrap();
    assert!(handles.is_empty());
}

#[test]
fn prepare_rejects_unknown_job_name() {
    let cron = CronConfig {
        enabled: true,
        tasks: vec![CronTaskSettings {
            job_name: "not_linked_job".into(),
            schedule: "0 0 * * * *".into(),
            ..Default::default()
        }],
    };
    let registry = JobRegistry::new();
    let Err(err) = CronScheduler::prepare(&cron, &registry) else {
        panic!("expected unknown job error");
    };
    assert!(err.to_string().contains("unknown job"));
}

#[test]
fn prepare_rejects_invalid_cron_expression() {
    let cron = CronConfig {
        enabled: true,
        tasks: vec![CronTaskSettings {
            job_name: "any".into(),
            schedule: "not a cron".into(),
            ..Default::default()
        }],
    };
    let mut registry = JobRegistry::new();
    registry.register("any", |_job, _ctx| Box::pin(async { Ok(()) }));
    let Err(err) = CronScheduler::prepare(&cron, &registry) else {
        panic!("expected invalid cron error");
    };
    assert!(err.to_string().contains("invalid cron"));
}

#[test]
fn yaml_cron_tasks_compile_when_job_registered() {
    let yaml = "jobs:\n  cron:\n    tasks:\n      - job_name: heartbeat\n        schedule: \"0 0 0 * * *\"\n";
    let cfg = JobsFileConfig::from_yaml(yaml).unwrap();
    let runtime = cfg
        .jobs
        .into_config_for(doido_core::Environment::Development);
    let mut registry = JobRegistry::new();
    registry.register("heartbeat", |_job, _ctx| Box::pin(async { Ok(()) }));
    let tasks = CronScheduler::prepare(&runtime.cron, &registry).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].job_name, "heartbeat");
}
