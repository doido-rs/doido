//! Azure Blob Storage [`Service`] (feature `storage-azure`).
//!
//! Implements the object operations (upload/download/delete/exists/size) via the
//! Azure SDK. It doesn't mint native SAS URLs yet, so blobs are served through
//! the app's proxy route (the trait's default `url`/`presigned_put` returning
//! `None`); native SAS is a follow-up.

use crate::config::ServiceConfig;
use crate::error::StorageError;
use crate::service::Service;
use azure_storage::prelude::*;
use azure_storage_blobs::prelude::*;
use doido_core::Result;

/// A [`Service`] backed by an Azure Blob Storage container.
pub struct AzureBlobService {
    name: String,
    container: ContainerClient,
    public: bool,
}

fn backend_err(e: impl std::fmt::Display) -> StorageError {
    StorageError::Backend(e.to_string())
}

impl AzureBlobService {
    /// Connect to the container described by `cfg`. The access key comes from the
    /// config `access_key` or the `AZURE_STORAGE_ACCESS_KEY` environment variable.
    pub fn connect(name: &str, cfg: &ServiceConfig) -> Result<Self> {
        let account = cfg
            .account
            .clone()
            .ok_or_else(|| StorageError::Config("azure service requires `account`".into()))?;
        let container = cfg
            .container
            .clone()
            .ok_or_else(|| StorageError::Config("azure service requires `container`".into()))?;
        let key = cfg
            .access_key
            .clone()
            .or_else(|| std::env::var("AZURE_STORAGE_ACCESS_KEY").ok())
            .ok_or_else(|| {
                StorageError::Config(
                    "azure service requires an access key (config `access_key` or \
                     AZURE_STORAGE_ACCESS_KEY)"
                        .into(),
                )
            })?;

        let credentials = StorageCredentials::access_key(account.clone(), key);
        let container = ClientBuilder::new(account, credentials).container_client(container);

        Ok(Self {
            name: name.to_string(),
            container,
            public: cfg.public,
        })
    }

    fn blob(&self, key: &str) -> BlobClient {
        self.container.blob_client(key)
    }
}

#[async_trait::async_trait]
impl Service for AzureBlobService {
    fn name(&self) -> &str {
        &self.name
    }

    fn public(&self) -> bool {
        self.public
    }

    async fn upload(&self, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<()> {
        let ct = content_type.unwrap_or("application/octet-stream").to_string();
        self.blob(key)
            .put_block_blob(data)
            .content_type(ct)
            .await
            .map_err(backend_err)?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        self.blob(key).get_content().await.map_err(|e| {
            // The SDK returns an error for a missing blob; surface NotFound.
            StorageError::NotFound(format!("{key}: {e}")).into()
        })
    }

    async fn delete(&self, key: &str) -> Result<()> {
        // Idempotent: only delete if present.
        if self.blob(key).exists().await.map_err(backend_err)? {
            self.blob(key).delete().await.map_err(backend_err)?;
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.blob(key).exists().await.map_err(|e| backend_err(e).into())
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let props = self
            .blob(key)
            .get_properties()
            .await
            .map_err(backend_err)?;
        Ok(props.blob.properties.content_length)
    }
}
