//! Blob creation records metadata (checksum, content type, size) and purge
//! removes both the object and its row.

use doido_storage::{checksum, MemoryService, Signer, Storage};
use std::sync::Arc;

async fn storage() -> Storage {
    let conn = doido_model::sea_orm::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let s = Storage::new(
        conn,
        Arc::new(MemoryService::default()),
        Signer::new(b"test-secret".to_vec()),
    );
    s.ensure_tables().await.unwrap();
    s
}

#[tokio::test]
async fn create_and_upload_records_metadata() {
    let s = storage().await;
    let data = b"the quick brown fox".to_vec();
    let blob = s
        .create_and_upload("fox.txt", data.clone(), None)
        .await
        .unwrap();

    assert_eq!(blob.filename, "fox.txt");
    assert_eq!(blob.content_type.as_deref(), Some("text/plain"));
    assert_eq!(blob.byte_size, data.len() as i64);
    assert_eq!(blob.checksum.as_deref(), Some(checksum::md5_base64(&data).as_str()));
    assert!(blob.text());

    // Persisted and downloadable.
    let found = s.find_blob(&blob.key).await.unwrap().unwrap();
    assert_eq!(found.filename, "fox.txt");
    assert_eq!(s.download(&blob.key).await.unwrap(), data);
}

#[tokio::test]
async fn purge_removes_object_and_row() {
    let s = storage().await;
    let blob = s
        .create_and_upload("bye.txt", b"bye".to_vec(), None)
        .await
        .unwrap();

    s.purge(&blob.key).await.unwrap();
    assert!(s.find_blob(&blob.key).await.unwrap().is_none());
    assert!(!s.service().exists(&blob.key).await.unwrap());
}

#[tokio::test]
async fn content_type_predicates_reflect_detection() {
    let s = storage().await;
    let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a].to_vec();
    let blob = s.create_and_upload("logo.bin", png, None).await.unwrap();
    assert_eq!(blob.content_type.as_deref(), Some("image/png"));
    assert!(blob.image());
    assert!(!blob.text());
}
