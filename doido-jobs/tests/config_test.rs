use doido_jobs::config::{build_queue, Backend};
use doido_jobs::{JobPayload, JobQueue, JobsConfig};
use serde_json::json;
use std::time::Duration;

#[test]
fn backend_default_is_memory() {
    assert_eq!(Backend::default(), Backend::Memory);
}

#[test]
fn test_backend_parse() {
    assert_eq!(Backend::parse("memory").unwrap(), Backend::Memory);
    assert_eq!(Backend::parse("inmemory").unwrap(), Backend::Memory);
    assert_eq!(Backend::parse("in_memory").unwrap(), Backend::Memory);
    assert_eq!(Backend::parse("DB").unwrap(), Backend::Db);
    assert_eq!(Backend::parse("database").unwrap(), Backend::Db);
    assert_eq!(Backend::parse("sql").unwrap(), Backend::Db);
    assert_eq!(Backend::parse("redis").unwrap(), Backend::Redis);
    assert!(Backend::parse("bogus").is_err());
}

#[cfg(feature = "jobs-redis")]
#[tokio::test]
async fn build_redis_backend_requires_url() {
    let cfg = JobsConfig {
        backend: Backend::Redis,
        redis_url: None,
        ..JobsConfig::default()
    };
    let err = build_queue(&cfg)
        .await
        .err()
        .expect("missing url")
        .to_string();
    assert!(err.contains("url"), "got: {err}");
}

#[cfg(feature = "jobs-redis")]
#[tokio::test]
async fn build_redis_backend_connects_or_reports_error() {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".into());
    let cfg = JobsConfig {
        backend: Backend::Redis,
        redis_url: Some(url.clone()),
        ..JobsConfig::default()
    };
    match build_queue(&cfg).await {
        Ok(queue) => {
            queue
                .enqueue(JobPayload::new("default", json!({}), 1))
                .await
                .unwrap();
        }
        Err(e) => {
            // Invalid host still exercises the connect path.
            assert!(
                e.to_string().contains("redis") || e.to_string().contains("connect"),
                "got: {e}"
            );
        }
    }
}

#[cfg(feature = "jobs-db")]
#[tokio::test]
async fn build_db_queue_helper_wraps_connection() {
    use doido_jobs::config::build_db_queue;
    use doido_jobs::db::DbQueue;
    use doido_model::connect_with_url;

    let conn = connect_with_url("sqlite::memory:").await.unwrap();
    DbQueue::new(conn.clone()).migrate().await.unwrap();
    let queue = build_db_queue(conn);
    queue
        .enqueue(JobPayload::new("default", json!({}), 1))
        .await
        .unwrap();
}

#[cfg(feature = "jobs-db")]
#[tokio::test]
async fn build_db_queue_from_connection() {
    use doido_jobs::db::DbQueue;
    use doido_model::connect_with_url;

    let conn = connect_with_url("sqlite::memory:").await.unwrap();
    let queue = DbQueue::new(conn);
    queue.migrate().await.unwrap();
    queue
        .enqueue(JobPayload::new("default", json!({}), 1))
        .await
        .unwrap();
}

#[test]
fn test_engine_config_derivation() {
    let cfg = JobsConfig {
        queues: vec!["critical".into(), "default".into()],
        concurrency: 0, // clamped to at least 1
        ..JobsConfig::default()
    };
    let ec = cfg.engine_config();
    assert_eq!(
        ec.queues,
        vec!["critical".to_string(), "default".to_string()]
    );
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

#[cfg(feature = "jobs-redis")]
#[tokio::test]
async fn build_configured_redis_queue_or_errors() {
    use doido_jobs::config::build_configured_queue;

    let cfg = JobsConfig {
        backend: Backend::Redis,
        redis_url: Some(
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".into()),
        ),
        ..JobsConfig::default()
    };
    match build_configured_queue(&cfg).await {
        Ok(q) => {
            q.enqueue(JobPayload::new("default", json!({}), 1))
                .await
                .unwrap();
        }
        Err(e) => assert!(
            e.to_string().contains("redis") || e.to_string().contains("connect"),
            "got: {e}"
        ),
    }
}

#[tokio::test]
async fn test_build_db_without_connection_errors() {
    let cfg = JobsConfig {
        backend: Backend::Db,
        ..JobsConfig::default()
    };
    assert!(build_queue(&cfg).await.is_err());
}
