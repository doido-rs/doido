//! has_one / has_many attachment semantics and dependent purge.

use doido_storage::{MemoryService, Signer, Storage};
use std::sync::Arc;

async fn storage() -> Storage {
    let conn = sea_orm::Database::connect("sqlite::memory:")
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
async fn has_one_replaces_previous() {
    let s = storage().await;
    let a = s
        .create_and_upload("a.txt", b"a".to_vec(), None)
        .await
        .unwrap();
    let b = s
        .create_and_upload("b.txt", b"b".to_vec(), None)
        .await
        .unwrap();

    // Attach then replace via detach+attach (has_one semantics).
    s.attach("User", "1", "avatar", &a.key).await.unwrap();
    assert_eq!(
        s.one("User", "1", "avatar").await.unwrap().unwrap().key,
        a.key
    );

    s.detach("User", "1", "avatar").await.unwrap();
    s.attach("User", "1", "avatar", &b.key).await.unwrap();
    assert_eq!(
        s.one("User", "1", "avatar").await.unwrap().unwrap().key,
        b.key
    );
}

#[tokio::test]
async fn has_many_keeps_all() {
    let s = storage().await;
    let one = s
        .create_and_upload("1.txt", b"1".to_vec(), None)
        .await
        .unwrap();
    let two = s
        .create_and_upload("2.txt", b"2".to_vec(), None)
        .await
        .unwrap();
    s.attach("Post", "9", "photos", &one.key).await.unwrap();
    s.attach("Post", "9", "photos", &two.key).await.unwrap();

    let photos = s.many("Post", "9", "photos").await.unwrap();
    assert_eq!(photos.len(), 2);
    assert_eq!(photos[0].key, one.key);
    assert_eq!(photos[1].key, two.key);
}

#[tokio::test]
async fn purge_for_record_purges_dependent_blobs() {
    let s = storage().await;
    let one = s
        .create_and_upload("1.txt", b"1".to_vec(), None)
        .await
        .unwrap();
    let two = s
        .create_and_upload("2.txt", b"2".to_vec(), None)
        .await
        .unwrap();
    s.attach("Post", "9", "photos", &one.key).await.unwrap();
    s.attach("Post", "9", "photos", &two.key).await.unwrap();

    s.purge_for_record("Post", "9").await.unwrap();

    assert!(s.many("Post", "9", "photos").await.unwrap().is_empty());
    assert!(s.find_blob(&one.key).await.unwrap().is_none());
    assert!(s.find_blob(&two.key).await.unwrap().is_none());
}

#[tokio::test]
async fn replace_one_swaps_attachment() {
    let s = storage().await;
    let a = s
        .create_and_upload("a.txt", b"a".to_vec(), None)
        .await
        .unwrap();
    let b = s
        .create_and_upload("b.txt", b"b".to_vec(), None)
        .await
        .unwrap();

    doido_storage::attachments::replace_one(s.conn(), "avatar", "User", "1", &a.key)
        .await
        .unwrap();
    doido_storage::attachments::replace_one(s.conn(), "avatar", "User", "1", &b.key)
        .await
        .unwrap();

    let keys = doido_storage::attachments::attached_keys(s.conn(), "avatar", "User", "1")
        .await
        .unwrap();
    assert_eq!(keys, vec![b.key]);
}

#[tokio::test]
async fn detach_blob_removes_one_of_many() {
    let s = storage().await;
    let one = s
        .create_and_upload("1.txt", b"1".to_vec(), None)
        .await
        .unwrap();
    let two = s
        .create_and_upload("2.txt", b"2".to_vec(), None)
        .await
        .unwrap();
    s.attach("Post", "1", "docs", &one.key).await.unwrap();
    s.attach("Post", "1", "docs", &two.key).await.unwrap();

    doido_storage::attachments::detach_blob(s.conn(), "docs", "Post", "1", &two.key)
        .await
        .unwrap();

    let keys = doido_storage::attachments::attached_keys(s.conn(), "docs", "Post", "1")
        .await
        .unwrap();
    assert_eq!(keys, vec![one.key]);
}
