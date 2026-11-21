//! Resolved auth subject identity before the full user model is loaded.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A lightweight identity resolved by a strategy (session cookie, JWT, OAuth, …).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthIdentity {
    pub user_id: Value,
}

impl AuthIdentity {
    pub fn new(user_id: impl Into<Value>) -> Self {
        Self {
            user_id: user_id.into(),
        }
    }
}
