//! Per-environment storage configuration, loaded from the `storage` section of
//! `config/<env>.yml` — the analogue of Rails' `config/storage.yml` plus
//! `config.active_storage.service`.
//!
//! ```yaml
//! storage:
//!   service: local                 # the active service for this environment
//!   services:
//!     local:  { type: disk, root: storage }
//!     test:   { type: memory }
//!     amazon: { type: s3, bucket: my-bucket, region: us-east-1 }
//!     cloudflare:
//!       type: r2
//!       bucket: my-bucket
//!       endpoint: "https://<accountid>.r2.cloudflarestorage.com"
//!     azure:  { type: azure, container: my-container, account: my-account }
//! ```
//!
//! [`StorageConfig::build`] turns the selected service into a live
//! `Arc<dyn Service>`. `disk` and `memory` are always available; the `s3`/`r2`
//! and `azure` backends are behind the `storage-s3` / `storage-azure` cargo
//! features and produce a clear error when selected without their feature.

use crate::disk::DiskService;
use crate::error::StorageError;
use crate::memory::MemoryService;
use crate::service::Service;
use doido_core::Result;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::sync::Arc;

/// Which storage backend a named service uses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ServiceBackend {
    /// Local filesystem (the default).
    #[default]
    Disk,
    /// In-process memory (dev/test).
    Memory,
    /// AWS S3 (or any S3-compatible endpoint). Requires `storage-s3`.
    S3,
    /// Cloudflare R2 (S3-compatible). Requires `storage-s3`.
    R2,
    /// Azure Blob Storage. Requires `storage-azure`.
    Azure,
    /// Google Cloud Storage. Requires `storage-gcs`.
    Gcs,
    /// A custom adapter registered via [`crate::register_adapter`], keyed by the
    /// unrecognized `type` string.
    Custom(String),
}

impl ServiceBackend {
    /// Map a `type` string to a backend, treating anything unrecognized as a
    /// custom adapter kind.
    fn from_kind(kind: &str) -> Self {
        match kind {
            "disk" => ServiceBackend::Disk,
            "memory" => ServiceBackend::Memory,
            "s3" => ServiceBackend::S3,
            "r2" => ServiceBackend::R2,
            "azure" => ServiceBackend::Azure,
            "gcs" | "google" => ServiceBackend::Gcs,
            other => ServiceBackend::Custom(other.to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for ServiceBackend {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let kind = String::deserialize(deserializer)?;
        Ok(ServiceBackend::from_kind(&kind))
    }
}

/// One named service's settings. Fields are shared across backends; each backend
/// reads the ones it needs.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServiceConfig {
    /// Backend kind. YAML key is `type`.
    #[serde(default, rename = "type")]
    pub backend: ServiceBackend,
    /// Whether objects are publicly readable (affects URL generation).
    #[serde(default)]
    pub public: bool,

    /// Disk: root directory (default `storage`).
    #[serde(default)]
    pub root: Option<String>,

    /// S3/R2: bucket name.
    #[serde(default)]
    pub bucket: Option<String>,
    /// S3/R2: region (R2 uses `auto`).
    #[serde(default)]
    pub region: Option<String>,
    /// S3/R2: custom endpoint (required for R2 and S3-compatible stores).
    #[serde(default)]
    pub endpoint: Option<String>,
    /// S3/R2: access key id (else read from the environment).
    #[serde(default)]
    pub access_key_id: Option<String>,
    /// S3/R2: secret access key (else read from the environment).
    #[serde(default)]
    pub secret_access_key: Option<String>,

    /// Azure: container name.
    #[serde(default)]
    pub container: Option<String>,
    /// Azure: storage account name.
    #[serde(default)]
    pub account: Option<String>,
    /// Azure: account access key (else read from the environment).
    #[serde(default)]
    pub access_key: Option<String>,

    /// Any extra keys not matched above — available to custom adapters (e.g. a
    /// `token` or `api_url` for an external service).
    #[serde(flatten)]
    pub options: HashMap<String, serde_norway::Value>,
}

impl ServiceConfig {
    /// A custom option read as a string, e.g. `cfg.option_str("token")`.
    pub fn option_str(&self, key: &str) -> Option<&str> {
        self.options.get(key).and_then(|v| v.as_str())
    }

    /// Build this service as a live `Arc<dyn Service>` named `name`.
    pub async fn build(&self, name: &str) -> Result<Arc<dyn Service>> {
        match &self.backend {
            ServiceBackend::Disk => {
                let root = self.root.clone().unwrap_or_else(|| "storage".to_string());
                Ok(Arc::new(DiskService::new(name, root).public(self.public)))
            }
            ServiceBackend::Memory => Ok(Arc::new(MemoryService::new(name))),
            ServiceBackend::S3 => self.build_s3(name, false).await,
            ServiceBackend::R2 => self.build_s3(name, true).await,
            ServiceBackend::Azure => self.build_azure(name).await,
            ServiceBackend::Gcs => self.build_gcs(name).await,
            ServiceBackend::Custom(kind) => crate::registry::build_adapter(kind, name, self),
        }
    }

    #[cfg(feature = "storage-s3")]
    async fn build_s3(&self, name: &str, r2: bool) -> Result<Arc<dyn Service>> {
        Ok(Arc::new(crate::s3::S3Service::connect(name, self, r2)?))
    }

    #[cfg(not(feature = "storage-s3"))]
    async fn build_s3(&self, _name: &str, r2: bool) -> Result<Arc<dyn Service>> {
        let kind = if r2 { "r2" } else { "s3" };
        Err(StorageError::Config(format!(
            "storage backend '{kind}' selected in config but doido-storage was built \
             without the `storage-s3` feature"
        ))
        .into())
    }

    #[cfg(feature = "storage-azure")]
    async fn build_azure(&self, name: &str) -> Result<Arc<dyn Service>> {
        Ok(Arc::new(crate::azure::AzureBlobService::connect(
            name, self,
        )?))
    }

    #[cfg(not(feature = "storage-azure"))]
    async fn build_azure(&self, _name: &str) -> Result<Arc<dyn Service>> {
        Err(StorageError::Config(
            "storage backend 'azure' selected in config but doido-storage was built \
             without the `storage-azure` feature"
                .to_string(),
        )
        .into())
    }

    #[cfg(feature = "storage-gcs")]
    async fn build_gcs(&self, name: &str) -> Result<Arc<dyn Service>> {
        Ok(Arc::new(crate::gcs::GcsService::connect(name, self).await?))
    }

    #[cfg(not(feature = "storage-gcs"))]
    async fn build_gcs(&self, _name: &str) -> Result<Arc<dyn Service>> {
        Err(StorageError::Config(
            "storage backend 'gcs' selected in config but doido-storage was built \
             without the `storage-gcs` feature"
                .to_string(),
        )
        .into())
    }
}

/// The `storage` config section: named services plus the selected one.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StorageConfig {
    /// Name of the active service. Defaults to the first entry (or a disk service
    /// rooted at `storage` when no services are configured).
    #[serde(default)]
    pub service: Option<String>,
    /// The named services.
    #[serde(default)]
    pub services: HashMap<String, ServiceConfig>,
}

impl StorageConfig {
    /// Build the selected service. With no configuration at all, defaults to a
    /// disk service named `local` rooted at `storage`.
    pub async fn build(&self) -> Result<Arc<dyn Service>> {
        if self.services.is_empty() {
            return Ok(Arc::new(DiskService::new("local", "storage")));
        }
        let name = self
            .service
            .clone()
            .or_else(|| self.services.keys().next().cloned())
            .ok_or_else(|| StorageError::Config("no storage service selected".to_string()))?;
        self.build_named(&name).await
    }

    /// Build a specific named service.
    pub async fn build_named(&self, name: &str) -> Result<Arc<dyn Service>> {
        let cfg = self
            .services
            .get(name)
            .ok_or_else(|| StorageError::Config(format!("unknown storage service {name:?}")))?;
        cfg.build(name).await
    }
}

/// File shape for `config/<env>.yml`; only the `storage` section is read.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct YamlConfig {
    #[serde(default)]
    pub storage: StorageConfig,
}

impl YamlConfig {
    /// Load `config/<env>.yml` for the current environment.
    pub fn load() -> std::io::Result<Self> {
        Self::load_env(crate::environment::Environment::get_env())
    }

    /// Load `config/<env>.yml` for a specific environment.
    pub fn load_env(env: crate::environment::Environment) -> std::io::Result<Self> {
        let path = format!("config/{}.yml", env.as_str());
        let contents = std::fs::read_to_string(&path)?;
        Self::from_yaml(&contents)
    }

    /// Parse a [`YamlConfig`] from a YAML string.
    pub fn from_yaml(yaml: &str) -> std::io::Result<Self> {
        serde_norway::from_str(yaml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Load the current environment's [`StorageConfig`], falling back to the default
/// (disk) when the file is missing or has no `storage` section.
pub fn load() -> StorageConfig {
    YamlConfig::load().map(|c| c.storage).unwrap_or_default()
}
