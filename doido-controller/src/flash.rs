//! Flash messages: values set on one request and read on exactly the next one.
//!
//! The flash is carried between requests in a signed cookie (reusing
//! [`CookieSessionStore`]). Because the next request does not re-write it unless
//! new messages are set, a flash naturally lives for a single following request
//! (Rails' `flash[:notice]`).

use crate::session::{CookieSessionStore, Session};
use std::collections::BTreeMap;

/// A bag of one-shot messages keyed by name (e.g. `notice`, `alert`).
#[derive(Debug, Default, Clone)]
pub struct Flash {
    messages: BTreeMap<String, String>,
}

impl Flash {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a flash message under `key` (e.g. `flash.set("notice", "Saved!")`).
    pub fn set(&mut self, key: impl Into<String>, message: impl Into<String>) {
        self.messages.insert(key.into(), message.into());
    }

    /// Read a flash message by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.messages.get(key).map(String::as_str)
    }

    /// Whether there are no messages.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Iterate `(key, message)` pairs, e.g. to render them all in a layout.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.messages.iter()
    }

    /// Serialize and sign the flash into a cookie value for the next request.
    pub fn to_cookie(&self, store: &CookieSessionStore) -> String {
        let mut session = Session::with_id("flash");
        session.set("flash", &self.messages);
        store.encode(&session)
    }

    /// Read the flash from a signed cookie value, yielding an empty flash when
    /// the cookie is absent, malformed, or fails signature verification.
    pub fn from_cookie(store: &CookieSessionStore, raw: &str) -> Self {
        let messages = store
            .decode(raw)
            .and_then(|s| s.get::<BTreeMap<String, String>>("flash"))
            .unwrap_or_default();
        Self { messages }
    }
}
