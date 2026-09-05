//! `Context::storage` delegates to the process-global storage facade.

use doido_controller::Context;
use doido_storage::{set_storage, MemoryService, Signer, Storage};
use http::Request;
use std::sync::Arc;

#[tokio::test]
async fn storage_delegates_to_global() {
    let conn = doido_model::sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let installed = Storage::new(
        conn,
        Arc::new(MemoryService::default()),
        Signer::new(b"test-secret".to_vec()),
    );
    assert!(
        set_storage(installed.clone()).is_ok(),
        "first install succeeds"
    );

    let ctx =
        Context::from_request_parts(Request::builder().uri("/").body(()).unwrap().into_parts().0);
    assert_eq!(
        ctx.storage().conn().get_database_backend(),
        installed.conn().get_database_backend()
    );
}
