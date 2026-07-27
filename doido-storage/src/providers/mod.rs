//! Storage provider drivers — every concrete [`Service`](crate::Service) backend
//! kept together in one isolated module.
//!
//! `disk` and `memory` are always built; the external-provider drivers (`s3`/R2,
//! `azure`, `gcs`) are each behind their cargo feature. The rest of the crate (the
//! `Service` contract, config, registry, blobs, attachments, serving) never
//! depends on a specific driver — it only sees `Arc<dyn Service>`.

pub mod disk;
pub mod memory;

#[cfg(feature = "storage-azure")]
pub mod azure;
#[cfg(feature = "storage-gcs")]
pub mod gcs;
#[cfg(feature = "storage-s3")]
pub mod s3;
