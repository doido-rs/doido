//! DiskService round-trips bytes under a tempdir and is idempotent on delete.

use doido_storage::{DiskService, Service};

async fn disk() -> (tempfile::TempDir, DiskService) {
    let dir = tempfile::tempdir().unwrap();
    let svc = DiskService::new("local", dir.path().to_path_buf());
    (dir, svc)
}

#[tokio::test]
async fn upload_download_roundtrip() {
    let (_dir, svc) = disk().await;
    svc.upload("abcd1234", b"hello disk".to_vec(), Some("text/plain"))
        .await
        .unwrap();

    assert!(svc.exists("abcd1234").await.unwrap());
    assert_eq!(svc.download("abcd1234").await.unwrap(), b"hello disk");
    assert_eq!(svc.size("abcd1234").await.unwrap(), 10);
}

#[tokio::test]
async fn delete_is_idempotent_and_download_missing_errs() {
    let (_dir, svc) = disk().await;
    // Deleting a missing key is fine.
    svc.delete("nope").await.unwrap();
    assert!(!svc.exists("nope").await.unwrap());
    assert!(svc.download("nope").await.is_err());

    svc.upload("k1", b"x".to_vec(), None).await.unwrap();
    svc.delete("k1").await.unwrap();
    assert!(!svc.exists("k1").await.unwrap());
}

#[tokio::test]
async fn rejects_keys_that_escape_the_root() {
    let (_dir, svc) = disk().await;
    assert!(svc.upload("../evil", b"x".to_vec(), None).await.is_err());
    assert!(svc.download("a/b").await.is_err());
}

#[tokio::test]
async fn public_flag_root_empty_key_and_short_shard() {
    let (dir, _svc) = disk().await;
    let public = DiskService::new("local", dir.path()).public(true);
    assert!(doido_storage::Service::public(&public));

    assert_eq!(public.root(), dir.path());
    assert!(public.upload("", b"x".to_vec(), None).await.is_err());
    assert!(public.size("missing").await.is_err());

    public.upload("ab", b"short".to_vec(), None).await.unwrap();
    let path = dir.path().join("ab").join("__").join("ab");
    assert!(path.exists());
}
