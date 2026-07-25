use doido_cable::heartbeat::{ping, ping_now};
use doido_cable::protocol::ServerFrame;

#[test]
fn ping_carries_a_timestamp_and_serializes() {
    match ping(1234) {
        ServerFrame::Ping { message } => assert_eq!(message, 1234),
        other => panic!("expected ping, got {other:?}"),
    }
    let json = ping(1).to_json().unwrap();
    assert!(json.contains(r#""type":"ping""#), "{json}");
}

#[test]
fn ping_now_uses_current_time() {
    match ping_now() {
        ServerFrame::Ping { message } => assert!(message > 0),
        other => panic!("expected ping, got {other:?}"),
    }
}
