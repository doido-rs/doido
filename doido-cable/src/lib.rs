pub mod cable;
pub mod channel;
pub mod protocol;
pub mod pubsub;
#[cfg(feature = "cable-redis")]
pub mod redis_pubsub;

pub use cable::Cable;
pub use channel::{Channel, ChannelContext, ChannelName};
// The `#[channel]` attribute macro. It lives in the macro namespace, so it
// coexists with the `channel` module (type namespace); `channel_macro` is kept
// as an alias for callers that prefer the unambiguous name.
pub use doido_cable_macros::channel;
pub use doido_cable_macros::channel as channel_macro;
pub use protocol::{CableFrame, ServerFrame, ServerMessage};
pub use pubsub::{MemoryPubSub, PubSub};
#[cfg(feature = "cable-redis")]
pub use redis_pubsub::RedisPubSub;
