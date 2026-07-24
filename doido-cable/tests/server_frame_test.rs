//! Round-trips and wire-format assertions for the ActionCable server→client
//! frames (docs/12-cable.md): welcome, ping, confirm/reject subscription, and
//! the untyped broadcast frame `{ "identifier": ..., "message": ... }`.

use doido_cable::{ServerFrame, ServerMessage};
use serde_json::json;

#[test]
fn welcome_frame_wire_format() {
    assert_eq!(
        ServerFrame::Welcome.to_json().unwrap(),
        r#"{"type":"welcome"}"#
    );
    assert_eq!(
        ServerFrame::parse(r#"{"type":"welcome"}"#).unwrap(),
        ServerFrame::Welcome
    );
}

#[test]
fn ping_frame_roundtrips() {
    let f = ServerFrame::Ping {
        message: 1_718_000_000,
    };
    let j = f.to_json().unwrap();
    assert!(j.contains(r#""type":"ping""#), "{j}");
    assert_eq!(ServerFrame::parse(&j).unwrap(), f);
}

#[test]
fn confirm_subscription_wire_format() {
    let f = ServerFrame::ConfirmSubscription {
        identifier: "ChatChannel".into(),
    };
    let j = f.to_json().unwrap();
    assert!(j.contains(r#""type":"confirm_subscription""#), "{j}");
    assert!(j.contains(r#""identifier":"ChatChannel""#), "{j}");
    assert_eq!(ServerFrame::parse(&j).unwrap(), f);
}

#[test]
fn reject_subscription_roundtrips() {
    let f = ServerFrame::RejectSubscription {
        identifier: "ChatChannel".into(),
    };
    assert_eq!(ServerFrame::parse(&f.to_json().unwrap()).unwrap(), f);
}

#[test]
fn broadcast_message_frame_has_no_type_tag() {
    let m = ServerMessage::new("ChatChannel", json!({ "content": "hello" }));
    let j = m.to_json().unwrap();
    assert!(
        !j.contains(r#""type""#),
        "broadcast frame must not carry a type tag: {j}"
    );
    assert!(j.contains(r#""identifier":"ChatChannel""#), "{j}");
    assert_eq!(ServerMessage::parse(&j).unwrap(), m);
}
