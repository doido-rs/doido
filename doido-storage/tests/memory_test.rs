//! MemoryService satisfies the Service contract in-process.

use doido_storage::{MemoryService, Service};

#[tokio::test]
async fn upload_download_delete() {
    let svc = MemoryService::new("test");
    assert_eq!(svc.name(), "test");

    svc.upload("k", b"in memory".to_vec(), Some("text/plain"))
        .await
        .unwrap();
    assert!(svc.exists("k").await.unwrap());
    assert_eq!(svc.size("k").await.unwrap(), 9);
    assert_eq!(svc.download("k").await.unwrap(), b"in memory");

    svc.delete("k").await.unwrap();
    assert!(!svc.exists("k").await.unwrap());
    assert!(svc.download("k").await.is_err());
}

#[tokio::test]
async fn no_native_url() {
    let svc = MemoryService::default();
    let opts = doido_storage::UrlOptions::default();
    assert!(svc.url("k", &opts).await.unwrap().is_none());
    assert!(svc.presigned_put("k", &opts).await.unwrap().is_none());
}
