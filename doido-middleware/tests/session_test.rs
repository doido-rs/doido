use doido_middleware::session::{Session, SessionStore};

struct FakeStore;

#[async_trait::async_trait]
impl SessionStore for FakeStore {
    async fn load(&self, _id: &str) -> doido_core::Result<Option<Session>> {
        Ok(None)
    }
    async fn save(&self, _session: &Session) -> doido_core::Result<()> {
        Ok(())
    }
    async fn destroy(&self, _id: &str) -> doido_core::Result<()> {
        Ok(())
    }
}

#[test]
fn test_session_store_trait_is_object_safe() {
    let _store: &dyn SessionStore = &FakeStore;
    // just checking it compiles as a trait object
}

#[test]
fn test_session_has_id_and_data() {
    let s = Session {
        id: "abc".to_string(),
        data: serde_json::json!({"k": "v"}),
    };
    assert_eq!(s.id, "abc");
}
