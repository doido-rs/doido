use doido_jobs::config::{build_queue, Backend};
use doido_jobs::{JobPayload, JobsConfig};
use serde_json::json;
use std::time::Duration;

#[test]
fn test_backend_parse() {
    assert_eq!(Backend::parse("memory").unwrap(), Backend::Memory);
    assert_eq!(Backend::parse("DB").unwrap(), Backend::Db);
    assert_eq!(Backend::parse("redis").unwrap(), Backend::Redis);
    assert!(Backend::parse("bogus").is_err());
}

#[test]
fn test_engine_config_derivation() {
    let cfg = JobsConfig {
        queues: vec!["critical".into(), "default".into()],
        concurrency: 0, // clamped to at least 1
        ..JobsConfig::default()
    };
    let ec = cfg.engine_config();
    assert_eq!(ec.queues, vec!["critical".to_string(), "default".to_string()]);
    assert_eq!(ec.concurrency, 1);
}

#[tokio::test]
async fn test_build_memory_queue_is_usable() {
    let cfg = JobsConfig::default();
    let queue = build_queue(&cfg).await.unwrap();
    queue
        .enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    let r = queue
        .reserve(&["default"], Duration::from_millis(50))
        .await
        .unwrap();
    assert!(r.is_some());
}

#[tokio::test]
async fn test_build_db_without_connection_errors() {
    let cfg = JobsConfig {
        backend: Backend::Db,
        ..JobsConfig::default()
    };
    assert!(build_queue(&cfg).await.is_err());
}
