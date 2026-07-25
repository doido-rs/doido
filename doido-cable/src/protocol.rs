use serde::{Deserialize, Serialize};

// Client→server frames are tagged with `command` per the ActionCable wire
// protocol (subscribe/unsubscribe/message); `type` is reserved for the
// server→client frames modelled by `ServerFrame`/`ServerMessage`.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "command", rename_all = "lowercase")]
pub enum CableFrame {
    Subscribe {
        identifier: String,
    },
    Unsubscribe {
        identifier: String,
    },
    Message {
        identifier: String,
        data: serde_json::Value,
    },
}

impl CableFrame {
    pub fn parse(json: &str) -> doido_core::Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| doido_core::anyhow::anyhow!("invalid cable frame: {e}"))
    }

    pub fn to_json(&self) -> doido_core::Result<String> {
        serde_json::to_string(self)
            .map_err(|e| doido_core::anyhow::anyhow!("cable frame serialize error: {e}"))
    }
}

/// Server→client control frames — the ActionCable `type`-tagged messages
/// (`welcome`, `ping`, `confirm_subscription`, `reject_subscription`).
///
/// The broadcast frame that carries channel data has no `type` tag and is
/// modelled separately as [`ServerMessage`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerFrame {
    Welcome,
    Ping { message: i64 },
    ConfirmSubscription { identifier: String },
    RejectSubscription { identifier: String },
}

impl ServerFrame {
    pub fn parse(json: &str) -> doido_core::Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| doido_core::anyhow::anyhow!("invalid server frame: {e}"))
    }

    pub fn to_json(&self) -> doido_core::Result<String> {
        serde_json::to_string(self)
            .map_err(|e| doido_core::anyhow::anyhow!("server frame serialize error: {e}"))
    }
}

/// Server→client broadcast frame: channel data delivered to subscribers. Per the
/// ActionCable wire protocol it carries no `type` tag —
/// `{ "identifier": "...", "message": {...} }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerMessage {
    pub identifier: String,
    pub message: serde_json::Value,
}

impl ServerMessage {
    pub fn new(identifier: impl Into<String>, message: serde_json::Value) -> Self {
        Self {
            identifier: identifier.into(),
            message,
        }
    }

    pub fn parse(json: &str) -> doido_core::Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| doido_core::anyhow::anyhow!("invalid server message: {e}"))
    }

    pub fn to_json(&self) -> doido_core::Result<String> {
        serde_json::to_string(self)
            .map_err(|e| doido_core::anyhow::anyhow!("server message serialize error: {e}"))
    }
}
