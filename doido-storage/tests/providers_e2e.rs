//! Provider e2e: upload/download round-trips against disk and local emulators.
//!
//! Cloud tests self-skip when emulators are offline so `cargo test --workspace`
//! (via `make verify`) keeps working without Docker. Run against live emulators
//! with `make test-storage-backends` after `make services-up`.

mod common;

use common::{service_roundtrip, unique_key};
use doido_storage::DiskService;

#[tokio::test]
async fn disk_upload_download_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let svc = DiskService::new("e2e-disk", dir.path().to_path_buf());
    let key = unique_key("disk");
    service_roundtrip(&svc, &key, b"hello disk e2e").await;
}

#[cfg(feature = "storage-s3")]
#[tokio::test]
async fn s3_localstack_upload_download_roundtrip() {
    let Some(svc) = common::s3::service().await else {
        return;
    };
    let key = unique_key("s3/");
    service_roundtrip(&svc, &key, b"hello localstack s3").await;
}

#[cfg(feature = "storage-azure")]
#[tokio::test]
async fn azure_azurite_upload_download_roundtrip() {
    let Some(svc) = common::azure::service().await else {
        return;
    };
    let key = unique_key("azure/");
    service_roundtrip(&svc, &key, b"hello azurite blob").await;
}

#[cfg(feature = "storage-gcs")]
#[tokio::test]
async fn gcs_fake_server_upload_download_roundtrip() {
    let Some(svc) = common::gcs::service().await else {
        return;
    };
    let key = unique_key("gcs/");
    service_roundtrip(&svc, &key, b"hello fake gcs").await;
}

#[cfg(feature = "storage-s3")]
#[tokio::test]
async fn storage_facade_s3_roundtrip() {
    common::storage_facade_s3_roundtrip().await;
}
