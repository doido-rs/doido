use doido_cable::CableFrame;

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
