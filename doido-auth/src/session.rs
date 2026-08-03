//! Cookie/session strategy — integrates with `doido_controller::session`.

use crate::config::AuthConfig;
use crate::identity::AuthIdentity;
use crate::strategy::AuthStrategy;
use async_trait::async_trait;
use doido_controller::session::{EncryptedCookieSessionStore, Session};
use doido_core::Result;
use doido_model::sea_orm::DatabaseConnection;
use http::header;
use http::request::Parts;

/// Session data key for the authenticated user's id.
pub const USER_ID_KEY: &str = "user_id";

/// Cookie name for the encrypted session (matches `doido_controller::context`).
pub const SESSION_COOKIE: &str = "_doido_session";

/// Cookie/session strategy using the encrypted cookie store.
pub struct SessionStrategy {
    store: EncryptedCookieSessionStore,
}

impl SessionStrategy {
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            store: EncryptedCookieSessionStore::new(secret),
        }
    }

    pub fn default_dev() -> Self {
        Self {
            store: EncryptedCookieSessionStore::default(),
        }
    }

    fn session_from_parts(&self, parts: &Parts) -> Option<Session> {
        let header = parts.headers.get(header::COOKIE)?.to_str().ok()?;
        let raw = header
            .split(';')
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(k, _)| *k == SESSION_COOKIE)
            .map(|(_, v)| v.to_string())?;
        self.store.decode(&raw)
    }
}

#[async_trait]
impl AuthStrategy for SessionStrategy {
    fn name(&self) -> &str {
        "cookie"
    }

    async fn authenticate(
        &self,
        parts: &Parts,
        _db: &DatabaseConnection,
    ) -> Result<Option<AuthIdentity>> {
        let session = match self.session_from_parts(parts) {
            Some(s) => s,
            None => return Ok(None),
        };
        let user_id = match session.data.get(USER_ID_KEY) {
            Some(value) => value.clone(),
            None => return Ok(None),
        };
        if user_id.is_null() {
            return Ok(None);
        }
        Ok(Some(AuthIdentity { user_id }))
    }
}

/// Store `user_id` in the session bag (call after successful sign-in).
pub fn sign_in_session(session: &mut Session, user_id: impl serde::Serialize) {
    session.set(USER_ID_KEY, user_id);
}

/// Clear the authenticated user from the session.
pub fn sign_out_session(session: &mut Session) {
    if session.data.is_object() {
        if let serde_json::Value::Object(map) = &mut session.data {
            map.remove(USER_ID_KEY);
        }
    }
}

/// Encode the session into a `Set-Cookie` value for `_doido_session`.
pub fn encode_session_cookie(store: &EncryptedCookieSessionStore, session: &Session) -> String {
    let value = store.encode(session);
    format!("{SESSION_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax")
}

/// Clear the session cookie in a response.
pub fn clear_session_cookie() -> String {
    format!("{SESSION_COOKIE}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
}

/// Build a session strategy from config (uses the process-global secret key base).
pub fn from_config(_config: &AuthConfig) -> SessionStrategy {
    SessionStrategy::new(doido_controller::secret::key_base())
}
