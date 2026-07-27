//! The local-filesystem [`Service`] — the default backend, analogous to Rails'
//! `Disk` service.
//!
//! Objects live under `root/<k0..2>/<k2..4>/<key>`, the same two-level fan-out
//! Rails uses to keep directories small. Disk has no native access URL, so
//! [`Service::url`] returns `None` and files are served through the app's signed
//! [`crate::serving`] routes.

use crate::error::StorageError;
use crate::service::Service;
use doido_core::Result;
use std::path::{Path, PathBuf};

/// A [`Service`] backed by a directory on the local filesystem.
pub struct DiskService {
    name: String,
    root: PathBuf,
    public: bool,
}

impl DiskService {
    /// Create a disk service rooted at `root`, named `name`.
    pub fn new(name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            root: root.into(),
            public: false,
        }
    }

    /// Mark objects as publicly readable (affects URL generation only).
    pub fn public(mut self, public: bool) -> Self {
        self.public = public;
        self
    }

    /// The sharded path for `key`, validating it can't escape the root.
    fn path_for(&self, key: &str) -> Result<PathBuf> {
        if key.is_empty() || key.contains('/') || key.contains('\\') || key.contains("..") {
            return Err(StorageError::Backend(format!("invalid storage key {key:?}")).into());
        }
        let (a, b) = shards(key);
        Ok(self.root.join(a).join(b).join(key))
    }
}

/// Two two-character shard directories derived from the key (padded for short keys).
fn shards(key: &str) -> (String, String) {
    let padded = format!("{key:_<4}");
    (padded[0..2].to_string(), padded[2..4].to_string())
}

#[async_trait::async_trait]
impl Service for DiskService {
    fn name(&self) -> &str {
        &self.name
    }

    fn public(&self) -> bool {
        self.public
    }

    async fn upload(&self, key: &str, data: Vec<u8>, _content_type: Option<&str>) -> Result<()> {
        let path = self.path_for(key)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(StorageError::from)?;
        }
        tokio::fs::write(&path, data)
            .await
            .map_err(StorageError::from)?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.path_for(key)?;
        match tokio::fs::read(&path).await {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(StorageError::NotFound(key.to_string()).into())
            }
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.path_for(key)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            // Idempotent: a missing object is not an error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let path = self.path_for(key)?;
        Ok(tokio::fs::try_exists(&path).await.unwrap_or(false))
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let path = self.path_for(key)?;
        let meta = tokio::fs::metadata(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(key.to_string())
            } else {
                StorageError::from(e)
            }
        })?;
        Ok(meta.len())
    }
}

/// The absolute root path (useful for tests and diagnostics).
impl DiskService {
    pub fn root(&self) -> &Path {
        &self.root
    }
}
