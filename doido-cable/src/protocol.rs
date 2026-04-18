use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CableFrame {
    Subscribe { identifier: String },
    Unsubscribe { identifier: String },
    Message { identifier: String, data: serde_json::Value },
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
