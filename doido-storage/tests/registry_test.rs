//! Custom adapters register and are selectable from config by their `type`.

use doido_storage::config::YamlConfig;
use doido_storage::{register_adapter, registered_adapters, MemoryService, Service, ServiceConfig};
use std::sync::Arc;

#[tokio::test]
async fn custom_adapter_selected_via_config_and_reads_options() {
    // A fake external adapter that just wraps MemoryService but proves it can read
    // arbitrary config options (e.g. an API token).
    register_adapter("dropbox_like", |name: &str, cfg: &ServiceConfig| {
        assert_eq!(cfg.option_str("token"), Some("secret-123"));
        assert_eq!(cfg.option_str("root"), None); // `root` is a typed field, not an option
        Ok(Arc::new(MemoryService::new(name)) as Arc<dyn Service>)
    });
    assert!(registered_adapters().contains(&"dropbox_like".to_string()));

    let yaml = "storage:\n  service: files\n  services:\n    files: { type: dropbox_like, token: secret-123 }\n";
    let cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    let svc = cfg.build().await.unwrap();

    assert_eq!(svc.name(), "files");
    svc.upload("k", b"hi".to_vec(), Some("text/plain"))
        .await
        .unwrap();
    assert_eq!(svc.download("k").await.unwrap(), b"hi");
}

#[tokio::test]
async fn unregistered_kind_errors_clearly() {
    let yaml = "storage:\n  service: x\n  services:\n    x: { type: totally_unknown_xyz }\n";
    let cfg = YamlConfig::from_yaml(yaml).unwrap().storage;
    let err = match cfg.build().await {
        Ok(_) => panic!("expected an error for an unregistered adapter"),
        Err(e) => e.to_string(),
    };
    assert!(
        err.contains("no storage adapter registered"),
        "unexpected error: {err}"
    );
    assert!(
        err.contains("totally_unknown_xyz"),
        "unexpected error: {err}"
    );
}
