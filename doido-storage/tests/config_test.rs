//! The `storage` YAML section parses and builds the selected service; cloud
//! backends error clearly when their feature is off.

use doido_storage::config::YamlConfig;
use doido_storage::{Service, ServiceBackend};

const YAML: &str = r#"
storage:
  service: test
  services:
    local: { type: disk, root: uploads }
    test:  { type: memory }
    amazon: { type: s3, bucket: my-bucket, region: us-east-1 }
"#;

#[test]
fn parses_named_services_and_selection() {
    let cfg = YamlConfig::from_yaml(YAML).unwrap().storage;
    assert_eq!(cfg.service.as_deref(), Some("test"));
    assert_eq!(cfg.services.len(), 3);
    assert_eq!(cfg.services["local"].backend, ServiceBackend::Disk);
    assert_eq!(cfg.services["local"].root.as_deref(), Some("uploads"));
    assert_eq!(cfg.services["test"].backend, ServiceBackend::Memory);
    assert_eq!(cfg.services["amazon"].backend, ServiceBackend::S3);
}

#[test]
fn parses_gcs_and_custom_backends() {
    let yaml = r#"
storage:
  services:
    google: { type: gcs, bucket: b }
    files:  { type: dropbox, token: xyz }
"#;
    let cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    assert_eq!(cfg.services["google"].backend, ServiceBackend::Gcs);
    assert_eq!(
        cfg.services["files"].backend,
        ServiceBackend::Custom("dropbox".to_string())
    );
    // Unmatched keys land in `options`.
    assert_eq!(cfg.services["files"].option_str("token"), Some("xyz"));
}

#[tokio::test]
async fn gcs_without_feature_errors_clearly() {
    let yaml = "storage:\n  service: g\n  services:\n    g: { type: gcs, bucket: b }\n";
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
async fn builds_the_selected_service() {
    let cfg = YamlConfig::from_yaml(YAML).unwrap().storage;
    let svc = cfg.build().await.unwrap();
    assert_eq!(svc.name(), "test"); // the `service: test` selection wins
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
