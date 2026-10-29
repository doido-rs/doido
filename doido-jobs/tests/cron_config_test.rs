use doido_jobs::config::JobsFileConfig;

#[test]
fn parses_cron_tasks_under_jobs() {
    let yaml = r#"
jobs:
  type: memory
  cron:
    enabled: true
    tasks:
      - job_name: sync_daily
        schedule: "0 0 3 * * *"
        payload: {"skip_reschedule": true}
        queue: catalog
"#;
    let file = JobsFileConfig::from_yaml(yaml).unwrap();
    let cron = file.jobs.cron.unwrap();
    assert_eq!(cron.enabled, Some(true));
    assert_eq!(cron.tasks.len(), 1);
    assert_eq!(cron.tasks[0].job_name, "sync_daily");
    assert_eq!(cron.tasks[0].schedule, "0 0 3 * * *");
    assert_eq!(cron.tasks[0].queue.as_deref(), Some("catalog"));
    assert_eq!(
        cron.tasks[0].payload,
        serde_json::json!({"skip_reschedule": true})
    );
}

#[test]
fn cron_defaults_to_enabled_when_tasks_present() {
    let cfg = JobsFileConfig::from_yaml(
        "jobs:\n  cron:\n    tasks:\n      - job_name: a\n        schedule: \"0 0 * * * *\"\n",
    )
    .unwrap()
    .jobs
    .into_config_for(doido_core::Environment::Development);
    assert!(cfg.cron.enabled);
    assert_eq!(cfg.cron.tasks.len(), 1);
}

#[test]
fn compile_skips_disabled_cron_tasks() {
    let cron = doido_jobs::cron::CronConfig {
        enabled: true,
        tasks: vec![
            doido_jobs::cron::CronTaskSettings {
                job_name: "on".into(),
                schedule: "0 0 * * * *".into(),
                enabled: true,
                ..Default::default()
            },
            doido_jobs::cron::CronTaskSettings {
                job_name: "off".into(),
                schedule: "0 0 * * * *".into(),
                enabled: false,
                ..Default::default()
            },
        ],
    };
    let mut registry = doido_jobs::JobRegistry::new();
    registry.register("on", |_job, _ctx| Box::pin(async { Ok(()) }));
    registry.register("off", |_job, _ctx| Box::pin(async { Ok(()) }));
    let compiled = cron.compile(&registry).unwrap();
    assert_eq!(compiled.len(), 1);
    assert_eq!(compiled[0].job_name, "on");
}

#[test]
fn cron_disabled_in_test_environment_by_default() {
    let cfg = JobsFileConfig::from_yaml(
        "jobs:\n  cron:\n    tasks:\n      - job_name: a\n        schedule: \"0 0 * * * *\"\n",
    )
    .unwrap()
    .jobs
    .into_config_for(doido_core::Environment::Test);
    assert!(!cfg.cron.enabled);
}
