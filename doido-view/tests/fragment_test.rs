use doido_cache::{CacheStore, MemoryStore};
use doido_view::fragment::cache_fragment;
use std::sync::Arc;

#[tokio::test]
async fn fragment_computed_on_miss_then_served_from_cache() {
    let store: Arc<dyn CacheStore> = Arc::new(MemoryStore::new());

    // Miss: render runs.
    let first = cache_fragment(&store, "sidebar", || "<nav>menu</nav>".to_string()).await;
    assert_eq!(first, "<nav>menu</nav>");

    // Hit: render must NOT run (panicking closure proves it).
    let second = cache_fragment(&store, "sidebar", || panic!("should not recompute")).await;
    assert_eq!(second, "<nav>menu</nav>");
}
