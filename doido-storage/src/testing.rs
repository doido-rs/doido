//! Test helpers for installing an in-memory storage facade.

use crate::client::Storage;
use crate::global::set_storage;
use crate::providers::memory::MemoryService;
use crate::signing::Signer;
use doido_model::sea_orm::DatabaseConnection;
use std::sync::Arc;

/// Build a [`Storage`] backed by [`MemoryService`] with a fixed test signer.
pub fn memory_storage(conn: DatabaseConnection) -> Storage {
    Storage::new(
        conn,
        Arc::new(MemoryService::default()),
        Signer::new(b"test-secret".to_vec()),
    )
}

/// Install `storage` as the process-global default. Returns `Err` with the
/// facade back if one was already installed.
pub fn install(storage: Storage) -> Result<(), Storage> {
    set_storage(storage)
}
