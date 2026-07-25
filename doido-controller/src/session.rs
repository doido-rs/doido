use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use doido_cache::CacheStore;
use doido_core::Result;
use hmac::{Hmac, KeyInit, Mac};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

/// A user session: an id plus a free-form JSON bag of values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub data: Value,
}

impl Session {
    /// A fresh session with a random id and an empty data bag.
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            data: Value::Object(Default::default()),
        }
    }

    /// A session with a caller-supplied id and an empty data bag.
    pub fn with_id(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            data: Value::Object(Default::default()),
        }
    }

    /// Store a serializable value under `key` (Rails `session[key] = value`).
    pub fn set(&mut self, key: &str, value: impl Serialize) {
        if !self.data.is_object() {
            self.data = Value::Object(Default::default());
        }
        if let Value::Object(map) = &mut self.data {
            map.insert(
                key.to_string(),
                serde_json::to_value(value).unwrap_or(Value::Null),
            );
        }
    }

    /// Read and deserialize the value under `key`, if present and well-typed.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Pluggable server-side session persistence. The cookie store keeps all state
/// in the signed cookie itself, so its `load` decodes that cookie and `save` is a
/// no-op; database/redis stores (see the `session-db`/`session-redis` features in
/// the spec) key server-side rows by the session id.
#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    async fn load(&self, id: &str) -> Result<Option<Session>>;
    async fn save(&self, session: &Session) -> Result<()>;
    async fn destroy(&self, id: &str) -> Result<()>;
}

/// The default, stateless session store: the whole session is serialized into a
/// cookie value and signed with HMAC-SHA256 so it cannot be tampered with. The
/// cookie value is `base64url(payload).base64url(signature)`.
///
/// (Spec 07 also calls for AES-256-GCM encryption on top of the signature; that
/// is a follow-up — this signs but does not yet encrypt the payload.)
pub struct CookieSessionStore {
    secret: Vec<u8>,
}

impl CookieSessionStore {
    /// Build a store that signs cookies with `secret` (e.g. `secret_key_base`).
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: secret.into(),
        }
    }

    /// Serialize and sign `session` into a `payload.signature` cookie value.
    pub fn encode(&self, session: &Session) -> String {
        let payload = serde_json::to_vec(session).unwrap_or_default();
        let msg = URL_SAFE_NO_PAD.encode(payload);
        let sig = self.sign(msg.as_bytes());
        format!("{msg}.{}", URL_SAFE_NO_PAD.encode(sig))
    }

    /// Verify and parse a signed cookie value. Returns `None` when the value is
    /// malformed, or the signature does not match (tampering or a wrong secret).
    pub fn decode(&self, raw: &str) -> Option<Session> {
        let (msg, sig_b64) = raw.split_once('.')?;
        let sig = URL_SAFE_NO_PAD.decode(sig_b64).ok()?;
        let mut mac = HmacSha256::new_from_slice(&self.secret).ok()?;
        mac.update(msg.as_bytes());
        // Constant-time comparison inside `verify_slice`.
        mac.verify_slice(&sig).ok()?;
        let payload = URL_SAFE_NO_PAD.decode(msg).ok()?;
        serde_json::from_slice(&payload).ok()
    }

    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts a key of any length");
        mac.update(msg);
        mac.finalize().into_bytes().to_vec()
    }
}

impl Default for CookieSessionStore {
    /// A dev-only store with a fixed, insecure secret. Real apps must call
    /// [`CookieSessionStore::new`] with a secret from config/credentials.
    fn default() -> Self {
        Self::new(b"doido-dev-insecure-secret".to_vec())
    }
}

#[async_trait::async_trait]
impl SessionStore for CookieSessionStore {
    /// Decode a signed cookie value (the "id" argument carries the raw cookie).
    async fn load(&self, raw: &str) -> Result<Option<Session>> {
        Ok(self.decode(raw))
    }

    /// No-op: a cookie session persists by having the response set its cookie
    /// (via [`CookieSessionStore::encode`]); there is no server-side state.
    async fn save(&self, _session: &Session) -> Result<()> {
        Ok(())
    }

    async fn destroy(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}

/// A server-side session store backed by any [`doido_cache::CacheStore`]: only
/// the session id travels in the cookie, while the data lives in the cache
/// (memory/redis/memcache). This is the generic backend the spec's
/// `DbSessionStore`/`RedisSessionStore` specialize.
pub struct CacheSessionStore {
    store: Arc<dyn CacheStore>,
    ttl_secs: Option<u64>,
}

impl CacheSessionStore {
    /// Persist sessions in `store` with no expiry.
    pub fn new(store: Arc<dyn CacheStore>) -> Self {
        Self {
            store,
            ttl_secs: None,
        }
    }

    /// Expire stored sessions after `secs` seconds of inactivity.
    pub fn with_ttl(mut self, secs: u64) -> Self {
        self.ttl_secs = Some(secs);
        self
    }

    fn key(id: &str) -> String {
        format!("session:{id}")
    }
}

#[async_trait::async_trait]
impl SessionStore for CacheSessionStore {
    async fn load(&self, id: &str) -> Result<Option<Session>> {
        match self.store.get(&Self::key(id)).await? {
            Some(value) => Ok(serde_json::from_value(value).ok()),
            None => Ok(None),
        }
    }

    async fn save(&self, session: &Session) -> Result<()> {
        let value = serde_json::to_value(session)?;
        self.store
            .set(&Self::key(&session.id), value, self.ttl_secs)
            .await
    }

    async fn destroy(&self, id: &str) -> Result<()> {
        self.store.delete(&Self::key(id)).await
    }
}
