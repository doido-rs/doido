use doido_cache::MemoryStore;
use doido_controller::session::{CacheSessionStore, Session, SessionStore};
use std::sync::Arc;

#[tokio::test]
async fn cache_session_store_round_trips_by_id() {
    let store = CacheSessionStore::new(Arc::new(MemoryStore::new()));
    let mut s = Session::with_id("sess-1");
    s.set("user_id", 7);

    store.save(&s).await.unwrap();
    let loaded = store
        .load("sess-1")
        .await
        .unwrap()
        .expect("session persisted server-side and loads back by id");
    assert_eq!(loaded.get::<i64>("user_id"), Some(7));
    assert_eq!(loaded.id, "sess-1");
}

#[tokio::test]
async fn cache_session_store_destroy_removes_it() {
    let store = CacheSessionStore::new(Arc::new(MemoryStore::new()));
    let s = Session::with_id("sess-2");
    store.save(&s).await.unwrap();
    store.destroy("sess-2").await.unwrap();
    assert!(store.load("sess-2").await.unwrap().is_none());
}

#[tokio::test]
async fn cache_session_store_missing_id_is_none() {
    let store = CacheSessionStore::new(Arc::new(MemoryStore::new()));
    assert!(store.load("never-saved").await.unwrap().is_none());
}
