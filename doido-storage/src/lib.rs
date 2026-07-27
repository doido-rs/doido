//! # doido-storage
//!
//! Attached-file storage for the Doido framework — the ActiveStorage analogue.
//!
//! It stores file bytes through a pluggable [`Service`] (disk by default, plus
//! in-memory, S3, Cloudflare R2 and Azure Blob behind features) and keeps
//! metadata (blobs, polymorphic attachments, variant records) in the database.
//! The [`Storage`] facade ties a service, a connection and a [`Signer`] together
//! and offers Rails-like operations; [`serving::routes`] mounts the blob-serving
//! and direct-upload endpoints on axum.
//!
//! ```no_run
//! # async fn demo(conn: doido_model::sea_orm::DatabaseConnection) -> doido_core::Result<()> {
//! use doido_storage::{Storage, MemoryService, Signer};
//! use std::sync::Arc;
//!
//! let storage = Storage::new(conn, Arc::new(MemoryService::default()), Signer::from_env());
//! storage.ensure_tables().await?;
//! let blob = storage.create_and_upload("hello.txt", b"hi".to_vec(), None).await?;
//! let bytes = storage.download(&blob.key).await?;
//! assert_eq!(bytes, b"hi");
//! # Ok(()) }
//! ```

pub mod attachments;
pub mod blob;
pub mod checksum;
pub mod client;
pub mod config;
pub mod content_type;
pub mod disk;
pub mod environment;
pub mod error;
pub mod memory;
pub mod registry;
pub mod schema;
pub mod service;
pub mod serving;
pub mod signing;

#[cfg(feature = "storage-image")]
pub mod analyzer;
#[cfg(feature = "storage-azure")]
pub mod azure;
#[cfg(feature = "storage-gcs")]
pub mod gcs;
#[cfg(feature = "storage-jobs")]
pub mod jobs;
#[cfg(feature = "storage-s3")]
pub mod s3;

pub use blob::Blob;
pub use client::Storage;
pub use config::{ServiceBackend, ServiceConfig, StorageConfig};
pub use disk::DiskService;
pub use environment::Environment;
pub use error::StorageError;
pub use memory::MemoryService;
pub use registry::{register_adapter, registered_adapters, ServiceFactory};
pub use schema::ensure_tables;
pub use service::{Service, UrlOptions};
pub use signing::{Disposition, Signer};

#[cfg(feature = "storage-azure")]
pub use azure::AzureBlobService;
#[cfg(feature = "storage-gcs")]
pub use gcs::GcsService;
#[cfg(feature = "storage-s3")]
pub use s3::S3Service;
