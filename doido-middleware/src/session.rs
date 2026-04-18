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
