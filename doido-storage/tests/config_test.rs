//! The `storage` YAML section parses and builds the selected driver; cloud
//! backends error clearly when their feature is off.

use doido_storage::config::YamlConfig;
use doido_storage::{Service, ServiceBackend};

const YAML: &str = r#"
storage:
  driver: test
  drivers:
    local: { type: disk, root: uploads }
    test:  { type: memory }
    amazon: { type: s3, bucket: my-bucket, region: us-east-1 }
"#;

#[test]
fn parses_named_drivers_and_selection() {
    let cfg = YamlConfig::from_yaml(YAML).unwrap().storage;
    assert_eq!(cfg.driver.as_deref(), Some("test"));
    assert_eq!(cfg.drivers.len(), 3);
    assert_eq!(cfg.drivers["local"].backend, ServiceBackend::Disk);
    assert_eq!(cfg.drivers["local"].root.as_deref(), Some("uploads"));
    assert_eq!(cfg.drivers["test"].backend, ServiceBackend::Memory);
    assert_eq!(cfg.drivers["amazon"].backend, ServiceBackend::S3);
}

#[test]
fn parses_legacy_service_keys_via_alias() {
    let yaml = r#"
storage:
  service: test
  services:
    test: { type: memory }
"#;
    let cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    assert_eq!(cfg.driver.as_deref(), Some("test"));
    assert_eq!(cfg.drivers["test"].backend, ServiceBackend::Memory);
}

#[test]
fn parses_gcs_and_custom_backends() {
    let yaml = r#"
storage:
  drivers:
    google: { type: gcs, bucket: b }
    files:  { type: dropbox, token: xyz }
"#;
    let cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    assert_eq!(cfg.drivers["google"].backend, ServiceBackend::Gcs);
    assert_eq!(
        cfg.drivers["files"].backend,
        ServiceBackend::Custom("dropbox".to_string())
    );
    // Unmatched keys land in `options`.
    assert_eq!(cfg.drivers["files"].option_str("token"), Some("xyz"));
}

#[tokio::test]
async fn gcs_without_feature_errors_clearly() {
    let yaml = "storage:\n  driver: g\n  drivers:\n    g: { type: gcs, bucket: b }\n";
    let cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    let result = cfg.build().await;
    #[cfg(not(feature = "storage-gcs"))]
    {
        let err = match result {
            Ok(_) => panic!("expected an error selecting gcs without the feature"),
            Err(e) => e.to_string(),
        };
        assert!(err.contains("storage-gcs"), "unexpected error: {err}");
    }
    #[cfg(feature = "storage-gcs")]
    {
        // With the feature on it may fail on missing credentials, but must not
        // complain about the feature being absent.
        if let Err(e) = result {
            assert!(!e.to_string().contains("without the `storage-gcs` feature"));
        }
    }
}

#[tokio::test]
async fn builds_the_selected_driver() {
    let cfg = YamlConfig::from_yaml(YAML).unwrap().storage;
    let svc = cfg.build().await.unwrap();
    assert_eq!(svc.name(), "test"); // the `driver: test` selection wins
}

#[tokio::test]
async fn empty_config_defaults_to_disk() {
    let cfg = doido_storage::StorageConfig::default();
    let svc = cfg.build().await.unwrap();
    assert_eq!(svc.name(), "local");
}

#[tokio::test]
async fn s3_without_feature_errors_clearly() {
    let cfg = YamlConfig::from_yaml(YAML).unwrap().storage;
    let result = cfg.build_named("amazon").await;
    #[cfg(not(feature = "storage-s3"))]
    {
        let err = match result {
            Ok(_) => panic!("expected an error selecting s3 without the feature"),
            Err(e) => e.to_string(),
        };
        assert!(err.contains("storage-s3"), "unexpected error: {err}");
    }
    #[cfg(feature = "storage-s3")]
    {
        // With the feature on, building may still fail on missing credentials,
        // but it must not complain about the feature being absent.
        if let Err(e) = result {
            assert!(!e.to_string().contains("without the `storage-s3` feature"));
        }
    }
}

#[tokio::test]
async fn build_named_missing_driver_errors() {
    let cfg = YamlConfig::from_yaml(YAML).unwrap().storage;
    let err = match cfg.build_named("missing").await {
        Ok(_) => panic!("expected missing driver error"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("missing") || err.contains("driver"));
}

#[tokio::test]
async fn disk_public_flag_propagates() {
    let yaml = r#"
storage:
  driver: pub
  drivers:
    pub: { type: disk, root: uploads, public: true }
"#;
    let cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    let svc = cfg.build().await.unwrap();
    assert!(svc.public());
}

#[test]
fn load_without_config_file_is_usable() {
    let cfg = doido_storage::config::load();
    assert!(cfg.drivers.is_empty() || cfg.driver.is_some() || cfg.drivers.contains_key("local"));
}

#[test]
fn apply_env_overrides_driver_prefix_and_expires() {
    let yaml = r#"
storage:
  driver: local
  drivers:
    local: { type: disk, root: storage }
    test:  { type: memory }
"#;
    let mut cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    std::env::set_var("STORAGE__DRIVER", "test");
    std::env::set_var("STORAGE__PREFIX", "/files");
    std::env::set_var("STORAGE__EXPIRES_IN", "900");
    cfg.apply_env_overrides();
    assert_eq!(cfg.driver.as_deref(), Some("test"));
    assert_eq!(cfg.resolved_prefix(), "/files");
    assert_eq!(cfg.resolved_expires_in().as_secs(), 900);
    std::env::remove_var("STORAGE__DRIVER");
    std::env::remove_var("STORAGE__PREFIX");
    std::env::remove_var("STORAGE__EXPIRES_IN");
}

#[test]
fn parses_prefix_and_expires_from_yaml() {
    let yaml = r#"
storage:
  driver: local
  prefix: /uploads
  expires_in: 120
  drivers:
    local: { type: memory }
"#;
    let cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    assert_eq!(cfg.resolved_prefix(), "/uploads");
    assert_eq!(cfg.resolved_expires_in().as_secs(), 120);
}

#[tokio::test]
async fn into_storage_propagates_prefix_and_expiry() {
    use doido_storage::{Signer, StorageConfig};
    let conn = doido_model::sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let mut cfg = StorageConfig::default();
    cfg.driver = Some("local".to_string());
    cfg.drivers.insert(
        "local".to_string(),
        doido_storage::ServiceConfig {
            backend: ServiceBackend::Memory,
            ..Default::default()
        },
    );
    cfg.prefix = Some("/custom".to_string());
    cfg.expires_in = Some(42);
    let storage = cfg
        .into_storage(conn, Signer::new(b"secret".to_vec()))
        .await
        .unwrap();
    assert_eq!(storage.prefix(), "/custom");
    assert_eq!(storage.expires_in().as_secs(), 42);
}
