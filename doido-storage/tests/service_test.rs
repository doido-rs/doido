use doido_storage::{DiskService, MemoryService, Service, UrlOptions};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn url_options_defaults() {
    let opts = UrlOptions::default();
    assert_eq!(opts.expires_in, Duration::from_secs(300));
    assert_eq!(opts.filename, None);
}

#[tokio::test]
async fn disk_service_default_url_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let svc = DiskService::new("local", dir.path());
    assert!(svc
        .url("key", &UrlOptions::default())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn arc_dyn_service_delegates() {
    let svc: Arc<dyn Service> = Arc::new(MemoryService::default());
    svc.upload("k", b"data".to_vec(), None).await.unwrap();
    assert_eq!(svc.download("k").await.unwrap(), b"data");
    assert_eq!(svc.name(), "memory");
}
