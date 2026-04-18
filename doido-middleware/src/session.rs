use serde_json::Value;
use doido_core::Result;

#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub data: Value,
}

pub trait SessionStore: Send + Sync {
    fn load(&self, id: &str) -> Result<Option<Session>>;
    fn save(&self, session: &Session) -> Result<()>;
    fn destroy(&self, id: &str) -> Result<()>;
}

pub struct CookieSessionStore;

impl SessionStore for CookieSessionStore {
    fn load(&self, _id: &str) -> Result<Option<Session>> { Ok(None) }
    fn save(&self, _session: &Session) -> Result<()> { Ok(()) }
    fn destroy(&self, _id: &str) -> Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::{Session, SessionStore};

    struct FakeStore;
    impl SessionStore for FakeStore {
        fn load(&self, _id: &str) -> doido_core::Result<Option<Session>> { Ok(None) }
        fn save(&self, _session: &Session) -> doido_core::Result<()> { Ok(()) }
        fn destroy(&self, _id: &str) -> doido_core::Result<()> { Ok(()) }
    }

    #[test]
    fn test_session_store_trait_is_object_safe() {
        let store: &dyn SessionStore = &FakeStore;
        assert!(store.load("x").unwrap().is_none());
    }

    #[test]
    fn test_session_has_id_and_data() {
        let s = Session { id: "abc".to_string(), data: serde_json::json!({"k": "v"}) };
        assert_eq!(s.id, "abc");
    }
}
