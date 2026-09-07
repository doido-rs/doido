//! Shared helpers for storage provider e2e tests against local emulators.

use doido_storage::Service;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

pub fn bucket_name() -> String {
    env_or("STORAGE_E2E_BUCKET", "doido-test")
}

/// Quick TCP probe — returns false when the emulator port is closed.
pub fn emulator_tcp_available(host: &str, port: u16) -> bool {
    let addr: SocketAddr = format!("{host}:{port}").parse().expect("valid host:port");
    TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok()
}

pub fn unique_key(prefix: &str) -> String {
    format!("{prefix}{}", uuid_simple())
}

fn uuid_simple() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{:016x}", n ^ std::process::id() as u64)
}

/// Upload → exists → download → size → delete → !exists.
pub async fn service_roundtrip(svc: &dyn Service, key: &str, bytes: &[u8]) {
    svc.upload(key, bytes.to_vec(), Some("application/octet-stream"))
        .await
        .expect("upload");
    assert!(svc.exists(key).await.expect("exists"));
    assert_eq!(svc.download(key).await.expect("download"), bytes);
    assert_eq!(svc.size(key).await.expect("size"), bytes.len() as u64);
    svc.delete(key).await.expect("delete");
    assert!(!svc.exists(key).await.expect("exists after delete"));
}

#[cfg(feature = "storage-s3")]
pub mod s3 {
    use super::*;
    use doido_storage::{S3Service, ServiceBackend, ServiceConfig};

    pub fn s3_config() -> ServiceConfig {
        ServiceConfig {
            backend: ServiceBackend::S3,
            bucket: Some(bucket_name()),
            region: Some("us-east-1".into()),
            endpoint: Some(env_or("STORAGE_E2E_S3_ENDPOINT", "http://127.0.0.1:4566")),
            access_key_id: Some("test".into()),
            secret_access_key: Some("test".into()),
            ..Default::default()
        }
    }

    pub async fn ensure_bucket(cfg: &ServiceConfig) {
        use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
        use aws_sdk_s3::Client;

        let bucket = cfg.bucket.as_ref().expect("bucket");
        let region = cfg.region.clone().unwrap_or_else(|| "us-east-1".into());
        let access_key = cfg.access_key_id.as_ref().expect("access_key_id");
        let secret = cfg.secret_access_key.as_ref().expect("secret_access_key");

        let mut builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(Credentials::new(access_key, secret, None, None, "e2e"));
        if let Some(endpoint) = &cfg.endpoint {
            builder = builder.endpoint_url(endpoint).force_path_style(true);
        }
        let client = Client::from_conf(builder.build());
        let _ = client.create_bucket().bucket(bucket).send().await;
    }

    pub async fn service() -> Option<S3Service> {
        if !emulator_tcp_available("127.0.0.1", 4566) {
            eprintln!("Floci/S3 emulator (127.0.0.1:4566) not reachable; skipping S3 e2e");
            return None;
        }
        let cfg = s3_config();
        ensure_bucket(&cfg).await;
        Some(S3Service::connect("e2e-s3", &cfg, false).expect("s3 connect"))
    }
}

#[cfg(feature = "storage-azure")]
pub mod azure {
    use super::*;
    use azure_storage_blobs::prelude::*;
    use doido_storage::{AzureBlobService, ServiceBackend, ServiceConfig};

    pub fn azure_config() -> ServiceConfig {
        ServiceConfig {
            backend: ServiceBackend::Azure,
            container: Some(bucket_name()),
            endpoint: Some(env_or(
                "STORAGE_E2E_AZURE_ENDPOINT",
                "http://127.0.0.1:10000/devstoreaccount1",
            )),
            ..Default::default()
        }
    }

    pub async fn ensure_container(container: &str) {
        let client = ClientBuilder::emulator().container_client(container);
        let _ = client.create().public_access(PublicAccess::None).await;
    }

    pub async fn service() -> Option<AzureBlobService> {
        if !emulator_tcp_available("127.0.0.1", 10000) {
            eprintln!("Azurite (127.0.0.1:10000) not reachable; skipping Azure e2e");
            return None;
        }
        let cfg = azure_config();
        let container = cfg.container.clone().expect("container");
        ensure_container(&container).await;
        Some(AzureBlobService::connect("e2e-azure", &cfg).expect("azure connect"))
    }
}

#[cfg(feature = "storage-gcs")]
pub mod gcs {
    use super::*;
    use doido_storage::{GcsService, ServiceBackend, ServiceConfig};

    pub fn gcs_config() -> ServiceConfig {
        ServiceConfig {
            backend: ServiceBackend::Gcs,
            bucket: Some(bucket_name()),
            endpoint: Some(env_or("STORAGE_E2E_GCS_ENDPOINT", "http://127.0.0.1:4443")),
            ..Default::default()
        }
    }

    pub async fn ensure_bucket(endpoint: &str, bucket: &str) {
        let url = format!("{endpoint}/storage/v1/b?project=doido-test");
        let body = serde_json::json!({ "name": bucket });
        let _ = reqwest::Client::new().post(&url).json(&body).send().await;
    }

    pub async fn service() -> Option<GcsService> {
        if !emulator_tcp_available("127.0.0.1", 4443) {
            eprintln!("fake-gcs-server (127.0.0.1:4443) not reachable; skipping GCS e2e");
            return None;
        }
        let cfg = gcs_config();
        let endpoint = cfg.endpoint.clone().expect("endpoint");
        let bucket = cfg.bucket.clone().expect("bucket");
        ensure_bucket(&endpoint, &bucket).await;
        Some(
            GcsService::connect("e2e-gcs", &cfg)
                .await
                .expect("gcs connect"),
        )
    }
}

#[cfg(feature = "storage-s3")]
pub async fn storage_facade_s3_roundtrip() {
    use doido_storage::{Signer, Storage};
    use std::sync::Arc;

    let Some(svc) = s3::service().await else {
        return;
    };
    let conn = doido_model::sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite");
    let storage = Storage::new(conn, Arc::new(svc), Signer::new(b"test-secret".to_vec()));
    storage.ensure_tables().await.expect("tables");
    let bytes = b"facade roundtrip via s3";
    let blob = storage
        .create_and_upload("facade.txt", bytes.to_vec(), None)
        .await
        .expect("create_and_upload");
    assert_eq!(storage.download(&blob.key).await.expect("download"), bytes);
}
