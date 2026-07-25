//! Cable connection identity + authorization (Rails `identified_by` /
//! `reject_unauthorized_connection`).
//!
//! A connection carries named identifiers (e.g. `current_user`) and is rejected
//! until it is authorized (e.g. after verifying a token in `connect`).

use std::collections::BTreeMap;

/// The identity of one WebSocket connection.
#[derive(Debug, Default, Clone)]
pub struct CableConnection {
    identifiers: BTreeMap<String, String>,
    authorized: bool,
}

impl CableConnection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach an identifier (Rails `self.current_user = ...`).
    pub fn identify(&mut self, key: &str, value: &str) -> &mut Self {
        self.identifiers.insert(key.to_string(), value.to_string());
        self
    }

    /// Read an identifier.
    pub fn identifier(&self, key: &str) -> Option<&str> {
        self.identifiers.get(key).map(String::as_str)
    }

    /// Mark the connection authorized (accept it).
    pub fn authorize(&mut self) -> &mut Self {
        self.authorized = true;
        self
    }

    /// Reject the connection (Rails `reject_unauthorized_connection`).
    pub fn reject(&mut self) -> &mut Self {
        self.authorized = false;
        self
    }

    /// Whether the connection has been authorized.
    pub fn is_authorized(&self) -> bool {
        self.authorized
    }
}
