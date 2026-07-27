//! The pluggable storage backend trait — the storage layer's analogue of
//! [`doido_cache::CacheStore`].
//!
//! A [`Service`] moves opaque bytes under a string `key`. Metadata (filename,
//! content type, checksum, byte size, attachments) lives in the database; the
//! service only stores and retrieves the raw object. Backends that can mint their
//! own access URLs (S3, Azure, R2) return them from [`Service::url`] /
//! [`Service::presigned_put`]; disk and memory return `None` and are served
//! through the app's own signed [`crate::serving`] routes.

use crate::signing::Disposition;
use doido_core::Result;
use std::sync::Arc;
use std::time::Duration;

/// Options controlling a generated access URL.
#[derive(Debug, Clone)]
pub struct UrlOptions {
    /// How long a signed/presigned URL stays valid.
    pub expires_in: Duration,
    /// `inline` (view in browser) or `attachment` (force download).
    pub disposition: Disposition,
    /// Suggested download filename (Content-Disposition).
    pub filename: Option<String>,
    /// Content type advertised for the object.
    pub content_type: Option<String>,
}

impl Default for UrlOptions {
    fn default() -> Self {
        Self {
            expires_in: Duration::from_secs(300),
            disposition: Disposition::Inline,
            filename: None,
            content_type: None,
        }
    }
}

/// A storage backend: stores and retrieves objects by key.
#[async_trait::async_trait]
pub trait Service: Send + Sync {
    /// Name of the configured service (e.g. `local`, `amazon`).
    fn name(&self) -> &str;

    /// Whether objects are publicly readable without signing.
    fn public(&self) -> bool {
        false
    }

    /// Store `data` under `key`, overwriting any existing object.
    async fn upload(&self, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<()>;

    /// Read the full object at `key`.
    async fn download(&self, key: &str) -> Result<Vec<u8>>;

    /// Delete the object at `key`. Idempotent: a missing object is not an error.
    async fn delete(&self, key: &str) -> Result<()>;

    /// Whether an object exists at `key`.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Size in bytes of the object at `key`.
    async fn size(&self, key: &str) -> Result<u64>;

    /// A backend-native URL to GET the object, if the backend can mint one
    /// (presigned for S3/Azure/R2, or a public URL). `None` means "no native
    /// URL — serve it through the app" (disk, memory).
    async fn url(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        let _ = (key, opts);
        Ok(None)
    }

    /// A backend-native URL a client may PUT to for a direct upload, if
    /// supported. `None` means "no native direct upload — use the app's disk
    /// upload route".
    async fn presigned_put(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        let _ = (key, opts);
        Ok(None)
    }
}

/// Lets a type-erased `Arc<dyn Service>` be used wherever a `Service` is expected,
/// mirroring `doido_cache`'s `impl CacheStore for Arc<dyn CacheStore>`.
#[async_trait::async_trait]
impl Service for Arc<dyn Service> {
    fn name(&self) -> &str {
        (**self).name()
    }
    fn public(&self) -> bool {
        (**self).public()
    }
    async fn upload(&self, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<()> {
        (**self).upload(key, data, content_type).await
    }
    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        (**self).download(key).await
    }
    async fn delete(&self, key: &str) -> Result<()> {
        (**self).delete(key).await
    }
    async fn exists(&self, key: &str) -> Result<bool> {
        (**self).exists(key).await
    }
    async fn size(&self, key: &str) -> Result<u64> {
        (**self).size(key).await
    }
    async fn url(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        (**self).url(key, opts).await
    }
    async fn presigned_put(&self, key: &str, opts: &UrlOptions) -> Result<Option<String>> {
        (**self).presigned_put(key, opts).await
    }
}
