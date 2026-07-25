use doido_cache::versioning::{read_versioned, versioned_key, write_versioned};
use doido_cache::MemoryStore;
use serde_json::json;

#[test]
fn versioned_key_format() {
    assert_eq!(versioned_key("post/1", 3), "post/1:v3");
}

#[tokio::test]
async fn stale_version_is_a_miss() {
    let store = MemoryStore::new();
    write_versioned(&store, "post/1", 1, json!("v1 body"), None)
        .await
        .unwrap();

    assert_eq!(
        read_versioned(&store, "post/1", 1).await.unwrap(),
        Some(json!("v1 body"))
    );
    // bumping the version invalidates cheaply — old key now misses
    assert_eq!(read_versioned(&store, "post/1", 2).await.unwrap(), None);
}
