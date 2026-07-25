use doido_cable::CableFrame;

#[test]
fn client_frames_serialize_with_the_command_tag() {
    let frame = CableFrame::Subscribe {
        identifier: "ChatChannel".to_string(),
    };
    let json = frame.to_json().unwrap();
    assert!(json.contains(r#""command":"subscribe""#), "{json}");
    assert!(
        !json.contains(r#""type""#),
        "client frames use `command`, not `type`"
    );

    // and round-trips back
    assert_eq!(CableFrame::parse(&json).unwrap(), frame);
}
