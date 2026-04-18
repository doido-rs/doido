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

#[cfg(test)]
mod tests {
    use super::CableFrame;

    #[test]
    fn test_parse_subscribe_frame() {
        let json = r#"{"type":"subscribe","identifier":"PostsChannel"}"#;
        let frame = CableFrame::parse(json).unwrap();
        assert_eq!(frame, CableFrame::Subscribe { identifier: "PostsChannel".to_string() });
    }

    #[test]
    fn test_parse_message_frame() {
        let json = r#"{"type":"message","identifier":"PostsChannel","data":{"action":"follow"}}"#;
        let frame = CableFrame::parse(json).unwrap();
        match frame {
            CableFrame::Message { identifier, data } => {
                assert_eq!(identifier, "PostsChannel");
                assert_eq!(data["action"], "follow");
            }
            _ => panic!("expected message frame"),
        }
    }

    #[test]
    fn test_roundtrip_unsubscribe_frame() {
        let frame = CableFrame::Unsubscribe { identifier: "ChatChannel".to_string() };
        let json = frame.to_json().unwrap();
        let parsed = CableFrame::parse(&json).unwrap();
        assert_eq!(frame, parsed);
    }
}
