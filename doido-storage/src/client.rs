//! [`Storage`] — the ergonomic facade over a service + database + signer.
//!
//! It bundles everything the storage layer needs (the configured
//! [`Service`](crate::Service), the sea-orm connection, and the [`Signer`]) and
//! offers Rails-like operations: `create_and_upload`, `attach`, `one`/`many`,
//! `purge`, `signed_id`, and URL helpers. It is `Clone` (cheap — the connection
//! and service are reference-counted) and doubles as the axum state for
//! [`crate::serving`].

use crate::attachments;
use crate::blob::{self, Blob};
use crate::config;
use crate::service::{Service, UrlOptions};
use crate::signing::{Disposition, Signer};
use doido_core::Result;
use sea_orm::DatabaseConnection;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Signing purpose for a blob `signed_id` embedded in serving URLs.
const PURPOSE_BLOB: &str = "blob";
/// Signing purpose for a disk direct-upload token.
pub(crate) const PURPOSE_DISK_UPLOAD: &str = "disk_upload";

/// The storage facade: a service + connection + signer with high-level helpers.
#[derive(Clone)]
pub struct Storage {
    conn: DatabaseConnection,
    service: Arc<dyn Service>,
    signer: Signer,
    prefix: String,
    expires_in: Duration,
}

impl Storage {
    /// Build a storage facade from an explicit service and signer.
    pub fn new(conn: DatabaseConnection, service: Arc<dyn Service>, signer: Signer) -> Self {
        Self {
            conn,
            service,
            signer,
            prefix: "/doido/storage".to_string(),
            expires_in: Duration::from_secs(300),
        }
    }

    /// Build from the `storage` config section (current environment) plus a signer
    /// read from `DOIDO_SECRET_KEY_BASE`.
    pub async fn from_config(conn: DatabaseConnection) -> Result<Self> {
        let service = config::load().build().await?;
        Ok(Self::new(conn, service, Signer::from_env()))
    }

    /// Override the URL route prefix (default `/doido/storage`).
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Override how long generated URLs stay valid (default 5 minutes).
    pub fn with_expiry(mut self, expires_in: Duration) -> Self {
        self.expires_in = expires_in;
        self
    }

    /// The sea-orm connection.
    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// The active storage service.
    pub fn service(&self) -> &Arc<dyn Service> {
        &self.service
    }

    /// The route prefix used by [`crate::serving::routes`].
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub(crate) fn signer(&self) -> &Signer {
        &self.signer
    }

    pub(crate) fn expires_in(&self) -> Duration {
        self.expires_in
    }

    /// Create the metadata tables if they don't exist (test/dev convenience).
    pub async fn ensure_tables(&self) -> Result<()> {
        crate::schema::ensure_tables(&self.conn).await
    }

    // --- Blobs -------------------------------------------------------------

    /// Upload `data` to the service and record a blob. Detects content type,
    /// computes the MD5 checksum and byte size.
    pub async fn create_and_upload(
        &self,
        filename: &str,
        data: Vec<u8>,
        metadata: Option<Value>,
    ) -> Result<Blob> {
        let blob = blob::build(filename, &data, self.service.name(), metadata);
        self.service
            .upload(&blob.key, data, blob.content_type.as_deref())
            .await?;
        blob::insert(&self.conn, &blob).await?;
        Ok(blob)
    }

    /// Find a blob by key.
    pub async fn find_blob(&self, key: &str) -> Result<Option<Blob>> {
        blob::find(&self.conn, key).await
    }

    /// Download a blob's bytes.
    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        self.service.download(key).await
    }

    /// Delete a blob everywhere: the service object, its attachment rows, and the
    /// metadata row.
    pub async fn purge(&self, key: &str) -> Result<()> {
        self.service.delete(key).await?;
        attachments::delete_for_blob(&self.conn, key).await?;
        blob::delete(&self.conn, key).await?;
        Ok(())
    }

    // --- Attachments -------------------------------------------------------

    /// Upload a file and attach it to `record` under `name` (append — for
    /// `has_many`). Returns the new blob.
    pub async fn attach_upload(
        &self,
        record_type: &str,
        record_id: &str,
        name: &str,
        filename: &str,
        data: Vec<u8>,
    ) -> Result<Blob> {
        let blob = self.create_and_upload(filename, data, None).await?;
        attachments::attach(&self.conn, name, record_type, record_id, &blob.key).await?;
        Ok(blob)
    }

    /// Attach an existing blob to `record` under `name` (append).
    pub async fn attach(
        &self,
        record_type: &str,
        record_id: &str,
        name: &str,
        blob_key: &str,
    ) -> Result<()> {
        attachments::attach(&self.conn, name, record_type, record_id, blob_key).await
    }

    /// `has_one_attached`: the attached blob, if any.
    pub async fn one(
        &self,
        record_type: &str,
        record_id: &str,
        name: &str,
    ) -> Result<Option<Blob>> {
        attachments::one(&self.conn, name, record_type, record_id).await
    }

    /// `has_many_attached`: all attached blobs.
    pub async fn many(&self, record_type: &str, record_id: &str, name: &str) -> Result<Vec<Blob>> {
        attachments::many(&self.conn, name, record_type, record_id).await
    }

    /// Detach (but don't purge) everything in the `name` slot.
    pub async fn detach(&self, record_type: &str, record_id: &str, name: &str) -> Result<()> {
        attachments::detach(&self.conn, name, record_type, record_id).await
    }

    /// Purge every blob attached to a record (the `dependent: :purge` default),
    /// then remove the attachment rows. Call from a model's after-destroy hook.
    pub async fn purge_for_record(&self, record_type: &str, record_id: &str) -> Result<()> {
        let keys = attachments::all_keys_for_record(&self.conn, record_type, record_id).await?;
        for key in keys {
            self.purge(&key).await?;
        }
        Ok(())
    }

    // --- Signing & URLs ----------------------------------------------------

    /// A permanent signed id for a blob key (Rails `blob.signed_id`).
    pub fn signed_id(&self, key: &str) -> String {
        self.signer.sign(key, PURPOSE_BLOB, None)
    }

    /// Resolve a signed id back to a blob key, erroring if tampered/expired.
    pub fn verify_signed_id(&self, signed_id: &str) -> Result<String> {
        self.signer.verify(signed_id, PURPOSE_BLOB)
    }

    /// The redirect URL for a blob (`{prefix}/blobs/redirect/{signed_id}/{filename}`)
    /// — the stable URL to put in HTML. The redirect handler 302s to the service's
    /// native URL (S3/Azure) or to the proxy route (disk/memory).
    pub fn redirect_path(&self, blob: &Blob) -> String {
        format!(
            "{}/blobs/redirect/{}/{}",
            self.prefix,
            self.signed_id(&blob.key),
            encode_filename(&blob.filename)
        )
    }

    /// The proxy URL for a blob (streams through the app).
    pub fn proxy_path(&self, blob: &Blob) -> String {
        format!(
            "{}/blobs/proxy/{}/{}",
            self.prefix,
            self.signed_id(&blob.key),
            encode_filename(&blob.filename)
        )
    }

    /// A URL a client can GET to fetch the object: the service's native
    /// (presigned/public) URL if it has one, else the proxy route.
    pub async fn url_for(&self, blob: &Blob, disposition: Disposition) -> Result<String> {
        let opts = UrlOptions {
            expires_in: self.expires_in,
            disposition,
            filename: Some(blob.filename.clone()),
            content_type: blob.content_type.clone(),
        };
        match self.service.url(&blob.key, &opts).await? {
            Some(url) => Ok(url),
            None => Ok(self.proxy_path(blob)),
        }
    }
}

/// Percent-ish-safe filename for a URL path segment (drops slashes).
fn encode_filename(filename: &str) -> String {
    filename.replace(['/', '\\'], "_")
}
