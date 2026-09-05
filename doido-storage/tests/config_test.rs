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
