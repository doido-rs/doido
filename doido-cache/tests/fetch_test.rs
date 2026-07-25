use doido_cache::fetch::fetch;
use doido_cache::MemoryStore;
use serde_json::json;

#[tokio::test]
async fn fetch_computes_on_miss_then_serves_from_cache() {
    let store = MemoryStore::new();

    let first = fetch(&store, "k", None, async || json!("computed"))
        .await
        .unwrap();
    assert_eq!(first, json!("computed"));

    // Hit: the render closure must not run.
    let second = fetch(&store, "k", None, async || panic!("should not recompute"))
        .await
        .unwrap();
    assert_eq!(second, json!("computed"));
}
