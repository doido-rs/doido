use doido_cable::config::{
    build_configured_pubsub, build_pubsub, Backend, CableConfig, CableFileConfig, CableSettings,
};
use std::time::Duration;

#[tokio::test]
async fn build_pubsub_memory_returns_working_backend() {
    let ps = build_pubsub(&CableConfig::default()).await.unwrap();
    let mut rx = ps.subscribe("room").await.unwrap();
    ps.publish("room", "hi").await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), "hi");
}

#[tokio::test]
async fn build_pubsub_db_requires_configured_builder() {
    let cfg = CableConfig {
        backend: Backend::Db,
        ..CableConfig::default()
    };
    let err = match build_pubsub(&cfg).await {
        Ok(_) => panic!("expected db backend error"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("build_configured_pubsub"));
}

#[tokio::test]
async fn build_configured_pubsub_db_without_feature_errors() {
    let cfg = CableConfig {
        backend: Backend::Db,
        ..CableConfig::default()
    };
    let err = match build_configured_pubsub(&cfg).await {
        Ok(_) => panic!("expected db backend error"),
        Err(e) => e.to_string(),
    };
    #[cfg(not(feature = "cable-db"))]
    assert!(err.contains("cable-db"));
}

#[tokio::test]
async fn build_pubsub_redis_without_url_or_feature_errors() {
    let cfg = CableConfig {
        backend: Backend::Redis,
        ..CableConfig::default()
    };
    let err = match build_pubsub(&cfg).await {
        Ok(_) => panic!("expected redis backend error"),
        Err(e) => e.to_string(),
    };
    #[cfg(not(feature = "cable-redis"))]
    assert!(err.contains("cable-redis"));
    #[cfg(feature = "cable-redis")]
    assert!(err.contains("url"));
}

#[tokio::test]
async fn pubsub_from_config_uses_memory_defaults() {
    let ps = doido_cable::pubsub_from_config().await.unwrap();
    ps.publish("x", "y").await.unwrap();
}

#[test]
fn ping_interval_zero_falls_back_to_default() {
    let cfg = CableSettings {
        ping_interval: Some(0),
        ..CableSettings::default()
    }
    .into_config();
    assert_eq!(cfg.ping_interval, Duration::from_secs(3));
}

#[test]
fn parses_db_backend_from_yaml() {
    let yaml = "cable:\n  type: db\n";
    let cfg = CableFileConfig::from_yaml(yaml)
        .unwrap()
        .cable
        .into_config();
    assert_eq!(cfg.backend, Backend::Db);
}
