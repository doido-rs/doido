use doido_cable::channel;

#[channel]
pub struct RoomChannel;

#[test]
fn channel_macro_generates_inherent_name_and_stream() {
    // Callable without importing the ChannelName trait.
    assert_eq!(RoomChannel::channel_name(), "RoomChannel");
    assert_eq!(RoomChannel::stream_name(), "RoomChannel");
}
