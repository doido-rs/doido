//! S3 and Cloudflare R2 [`Service`] (feature `storage-s3`).
//!
//! Both talk the S3 API via the official `aws-sdk-s3` crate; R2 (and any
//! S3-compatible store) is just S3 with a custom endpoint and path-style
//! addressing. Credentials come from the config or, if absent, the standard AWS
//! environment variables.

use crate::config::ServiceConfig;
use crate::error::StorageError;
use crate::service::{Service, UrlOptions};
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use doido_core::Result;

/// A [`Service`] backed by an S3 bucket (AWS S3, Cloudflare R2, or any
/// S3-compatible endpoint).
pub struct S3Service {
    name: String,
    client: Client,
    bucket: String,
    public: bool,
}

fn backend_err(e: impl std::fmt::Display) -> StorageError {
    StorageError::Backend(e.to_string())
}

/// Resolve credentials from the config, falling back to the standard AWS
/// environment variables (`AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`, plus an
/// optional `AWS_SESSION_TOKEN`).
fn resolve_credentials(cfg: &ServiceConfig) -> Result<Credentials> {
    if let (Some(a), Some(s)) = (&cfg.access_key_id, &cfg.secret_access_key) {
        return Ok(Credentials::new(a, s, None, None, "doido-config"));
    }
    match (
        std::env::var("AWS_ACCESS_KEY_ID"),
        std::env::var("AWS_SECRET_ACCESS_KEY"),
    ) {
        (Ok(a), Ok(s)) => Ok(Credentials::new(
            a,
            s,
            std::env::var("AWS_SESSION_TOKEN").ok(),
            None,
            "doido-env",
        )),
        _ => Err(StorageError::Config(
            "s3/r2 service requires credentials (config `access_key_id`/`secret_access_key` \
             or AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY)"
                .into(),
        )
        .into()),
    }
}

impl S3Service {
    /// Connect to the bucket described by `cfg`. When `r2` is set (or a custom
    /// endpoint is given) path-style addressing is used.
    pub fn connect(name: &str, cfg: &ServiceConfig, r2: bool) -> Result<Self> {
        let bucket = cfg
            .bucket
            .clone()
            .ok_or_else(|| StorageError::Config("s3/r2 service requires `bucket`".into()))?;

        let custom_endpoint = cfg.endpoint.is_some();
        let region = cfg.region.clone().unwrap_or_else(|| {
            if custom_endpoint {
                "auto".into()
            } else {
                "us-east-1".into()
            }
        });

        let mut builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(resolve_credentials(cfg)?);
        if let Some(endpoint) = &cfg.endpoint {
            builder = builder.endpoint_url(endpoint);
        }
        // R2 and other S3-compatible endpoints require path-style addressing.
        if r2 || custom_endpoint {
            builder = builder.force_path_style(true);
        }

        Ok(Self {
            name: name.to_string(),
            client: Client::from_conf(builder.build()),
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
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .content_type(ct)
            .send()
            .await
            .map_err(backend_err)?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let out = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                if e.as_service_error().is_some_and(|se| se.is_no_such_key()) {
                    StorageError::NotFound(key.to_string())
                } else {
                    backend_err(e)
                }
            })?;
        let data = out.body.collect().await.map_err(backend_err)?;
        Ok(data.into_bytes().to_vec())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(backend_err)?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        // A missing object surfaces as an error; treat any error as "absent",
        // matching the previous backend's lenient behavior.
        Ok(self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .is_ok())
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let head = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                if e.as_service_error().is_some_and(|se| se.is_not_found()) {
                    StorageError::NotFound(key.to_string())
                } else {
                    backend_err(e)
                }
            })?;
        Ok(head.content_length().unwrap_or(0).max(0) as u64)
    }

    async fn url(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        let config = PresigningConfig::expires_in(opts.expires_in).map_err(backend_err)?;
        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(backend_err)?;
        Ok(Some(req.uri().to_string()))
    }

    async fn presigned_put(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        let config = PresigningConfig::expires_in(opts.expires_in).map_err(backend_err)?;
        let req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(backend_err)?;
        Ok(Some(req.uri().to_string()))
    }
}
