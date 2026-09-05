//! `global::init` builds the facade from config and is idempotent.
//!
//! Separate test binary so it owns a fresh global `OnceLock`.

use doido_storage::{global, MemoryService, Signer, Storage};
use std::sync::Arc;

async fn test_storage() -> Storage {
    let conn = doido_model::sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    Storage::new(
        conn,
        Arc::new(MemoryService::default()),
        Signer::new(b"test-secret".to_vec()),
    )
}

#[tokio::test]
async fn set_storage_and_init_are_idempotent() {
    let s = test_storage().await;
    assert!(global::set_storage(s).is_ok(), "first install succeeds");
    assert!(global::try_storage().is_some());

    let again = global::init().await.unwrap();
    assert!(global::try_storage().is_some());
    drop(again);
}
