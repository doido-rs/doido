//! S3 and Cloudflare R2 [`Service`] (feature `storage-s3`).
//!
//! Both use the S3 API via the `rust-s3` crate; R2 (and any S3-compatible store)
//! is just S3 with a custom endpoint and path-style addressing. Credentials come
//! from the config or, if absent, the standard AWS environment variables.

use crate::config::ServiceConfig;
use crate::error::StorageError;
use crate::service::{Service, UrlOptions};
use doido_core::Result;
use s3::creds::Credentials;
use s3::region::Region;
use s3::Bucket;

/// A [`Service`] backed by an S3 bucket (AWS S3, Cloudflare R2, or any
/// S3-compatible endpoint).
pub struct S3Service {
    name: String,
    bucket: Box<Bucket>,
    public: bool,
}

fn backend_err(e: impl std::fmt::Display) -> StorageError {
    StorageError::Backend(e.to_string())
}

impl S3Service {
    /// Connect to the bucket described by `cfg`. When `r2` is set (or a custom
    /// endpoint is given) path-style addressing is used.
    pub fn connect(name: &str, cfg: &ServiceConfig, r2: bool) -> Result<Self> {
        let bucket_name = cfg
            .bucket
            .clone()
            .ok_or_else(|| StorageError::Config("s3/r2 service requires `bucket`".into()))?;

        let region = match &cfg.endpoint {
            Some(endpoint) => Region::Custom {
                region: cfg.region.clone().unwrap_or_else(|| "auto".into()),
                endpoint: endpoint.clone(),
            },
            None => cfg
                .region
                .clone()
                .unwrap_or_else(|| "us-east-1".into())
                .parse()
                .map_err(|e| StorageError::Config(format!("invalid s3 region: {e}")))?,
        };

        let creds = match (&cfg.access_key_id, &cfg.secret_access_key) {
            (Some(a), Some(s)) => Credentials::new(Some(a), Some(s), None, None, None),
            _ => Credentials::default(),
        }
        .map_err(|e| StorageError::Config(format!("s3 credentials: {e}")))?;

        let mut bucket = Bucket::new(&bucket_name, region, creds).map_err(backend_err)?;
        if r2 || cfg.endpoint.is_some() {
            bucket = bucket.with_path_style();
        }

        Ok(Self {
            name: name.to_string(),
            bucket,
            public: cfg.public,
        })
    }
}

#[async_trait::async_trait]
impl Service for S3Service {
    fn name(&self) -> &str {
        &self.name
    }

    fn public(&self) -> bool {
        self.public
    }

    async fn upload(&self, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<()> {
        let ct = content_type.unwrap_or("application/octet-stream");
        self.bucket
            .put_object_with_content_type(key, &data, ct)
            .await
            .map_err(backend_err)?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let resp = self.bucket.get_object(key).await.map_err(backend_err)?;
        if resp.status_code() == 404 {
            return Err(StorageError::NotFound(key.to_string()).into());
        }
        Ok(resp.bytes().to_vec())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.bucket.delete_object(key).await.map_err(backend_err)?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        match self.bucket.head_object(key).await {
            Ok((_, code)) => Ok(code == 200),
            Err(_) => Ok(false),
        }
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let (head, code) = self.bucket.head_object(key).await.map_err(backend_err)?;
        if code == 404 {
            return Err(StorageError::NotFound(key.to_string()).into());
        }
        Ok(head.content_length.unwrap_or(0) as u64)
    }

    async fn url(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        let secs = opts.expires_in.as_secs() as u32;
        let url = self
            .bucket
            .presign_get(key, secs, None)
            .await
            .map_err(backend_err)?;
        Ok(Some(url))
    }

    async fn presigned_put(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        let secs = opts.expires_in.as_secs() as u32;
        let url = self
            .bucket
            .presign_put(key, secs, None, None)
            .await
            .map_err(backend_err)?;
        Ok(Some(url))
    }
}
