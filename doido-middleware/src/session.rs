use serde_json::Value;
use doido_core::Result;

#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub data: Value,
}

#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    async fn load(&self, id: &str) -> Result<Option<Session>>;
    async fn save(&self, session: &Session) -> Result<()>;
    async fn destroy(&self, id: &str) -> Result<()>;
}

pub struct CookieSessionStore;

#[async_trait::async_trait]
impl SessionStore for CookieSessionStore {
    async fn load(&self, _id: &str) -> Result<Option<Session>> { Ok(None) }
    async fn save(&self, _session: &Session) -> Result<()> { Ok(()) }
    async fn destroy(&self, _id: &str) -> Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::{Session, SessionStore};

    struct FakeStore;

    #[async_trait::async_trait]
    impl SessionStore for FakeStore {
        async fn load(&self, _id: &str) -> doido_core::Result<Option<Session>> { Ok(None) }
        async fn save(&self, _session: &Session) -> doido_core::Result<()> { Ok(()) }
        async fn destroy(&self, _id: &str) -> doido_core::Result<()> { Ok(()) }
    }

    #[test]
    fn test_session_store_trait_is_object_safe() {
        let _store: &dyn SessionStore = &FakeStore;
        // just checking it compiles as a trait object
    }

    #[test]
    fn test_session_has_id_and_data() {
        let s = Session { id: "abc".to_string(), data: serde_json::json!({"k": "v"}) };
        assert_eq!(s.id, "abc");
    }
}
