//! Google Cloud Storage [`Service`] (feature `storage-gcs`).
//!
//! Uses the `gcloud-storage` client. Authentication is Application Default
//! Credentials — set `GOOGLE_APPLICATION_CREDENTIALS` to a service-account JSON
//! key, or run on GCP with an attached service account. URLs are V4 signed
//! (redirect mode); if signing is unavailable, blobs fall back to the app proxy.

use crate::config::ServiceConfig;
use crate::error::StorageError;
use crate::service::{Service, UrlOptions};
use doido_core::Result;
use gcloud_storage::client::{Client, ClientConfig};
use gcloud_storage::http::objects::delete::DeleteObjectRequest;
use gcloud_storage::http::objects::download::Range;
use gcloud_storage::http::objects::get::GetObjectRequest;
use gcloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use gcloud_storage::sign::{SignedURLMethod, SignedURLOptions};

/// A [`Service`] backed by a Google Cloud Storage bucket.
pub struct GcsService {
    name: String,
    client: Client,
    bucket: String,
    public: bool,
}

fn backend_err(e: impl std::fmt::Display) -> StorageError {
    StorageError::Backend(e.to_string())
}

impl GcsService {
    /// Connect to the bucket in `cfg` using Application Default Credentials.
    pub async fn connect(name: &str, cfg: &ServiceConfig) -> Result<Self> {
        let bucket = cfg
            .bucket
            .clone()
            .ok_or_else(|| StorageError::Config("gcs service requires `bucket`".into()))?;
        let config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|e| StorageError::Config(format!("gcs auth: {e}")))?;
        Ok(Self {
            name: name.to_string(),
            client: Client::new(config),
            bucket,
            public: cfg.public,
        })
    }

    async fn signed_url(&self, key: &str, method: SignedURLMethod, secs: u32) -> Result<String> {
        let opts = SignedURLOptions {
            method,
            expires: std::time::Duration::from_secs(secs as u64),
            ..Default::default()
        };
        self.client
            .signed_url(&self.bucket, key, None, None, opts)
            .await
            .map_err(|e| backend_err(e).into())
    }
}

#[async_trait::async_trait]
impl Service for GcsService {
    fn name(&self) -> &str {
        &self.name
    }

    fn public(&self) -> bool {
        self.public
    }

    async fn upload(&self, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<()> {
        let mut media = Media::new(key.to_string());
        if let Some(ct) = content_type {
            media.content_type = ct.to_string().into();
        }
        let upload_type = UploadType::Simple(media);
        let req = UploadObjectRequest {
            bucket: self.bucket.clone(),
            ..Default::default()
        };
        self.client
            .upload_object(&req, data, &upload_type)
            .await
            .map_err(backend_err)?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let req = GetObjectRequest {
            bucket: self.bucket.clone(),
            object: key.to_string(),
            ..Default::default()
        };
        self.client
            .download_object(&req, &Range::default())
            .await
            .map_err(|e| StorageError::NotFound(format!("{key}: {e}")).into())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let req = DeleteObjectRequest {
            bucket: self.bucket.clone(),
            object: key.to_string(),
            ..Default::default()
        };
        // Idempotent: ignore a "not found" delete.
        match self.client.delete_object(&req).await {
            Ok(()) => Ok(()),
            Err(_) if !self.exists(key).await.unwrap_or(true) => Ok(()),
            Err(e) => Err(backend_err(e).into()),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let req = GetObjectRequest {
            bucket: self.bucket.clone(),
            object: key.to_string(),
            ..Default::default()
        };
        Ok(self.client.get_object(&req).await.is_ok())
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let req = GetObjectRequest {
            bucket: self.bucket.clone(),
            object: key.to_string(),
            ..Default::default()
        };
        let obj = self.client.get_object(&req).await.map_err(|e| {
            if e.to_string().contains("404") {
                StorageError::NotFound(key.to_string())
            } else {
                backend_err(e)
            }
        })?;
        Ok(obj.size.max(0) as u64)
    }

    async fn url(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        let secs = opts.expires_in.as_secs() as u32;
        Ok(Some(
            self.signed_url(key, SignedURLMethod::GET, secs).await?,
        ))
    }

    async fn presigned_put(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        let secs = opts.expires_in.as_secs() as u32;
        Ok(Some(
            self.signed_url(key, SignedURLMethod::PUT, secs).await?,
        ))
    }
}
