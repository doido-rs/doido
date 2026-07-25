use doido_cable::streams::{stream_for, stream_from};

#[test]
fn stream_name_helpers() {
    assert_eq!(stream_from("chat_room_1"), "chat_room_1");
    assert_eq!(stream_for("Room", 7), "Room:7");
    assert_eq!(stream_for("Post", "abc"), "Post:abc");
}
