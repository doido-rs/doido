pub mod cable;
pub mod channel;
pub mod connection;
#[cfg(feature = "cable-db")]
pub mod db_pubsub;
pub mod heartbeat;
pub mod protocol;
pub mod pubsub;
#[cfg(feature = "cable-redis")]
pub mod redis_pubsub;
pub mod server;
pub mod streams;

pub use cable::Cable;
pub use channel::{Channel, ChannelContext, ChannelName};
pub use connection::CableConnection;
pub use server::{handle_socket, route as cable_route, ws_handler, ChannelRegistry};
// The `#[channel]` attribute macro. It lives in the macro namespace, so it
// coexists with the `channel` module (type namespace); `channel_macro` is kept
// as an alias for callers that prefer the unambiguous name.
#[cfg(feature = "cable-db")]
pub use db_pubsub::DbPubSub;
pub use doido_cable_macros::channel;
pub use doido_cable_macros::channel as channel_macro;
pub use protocol::{CableFrame, ServerFrame, ServerMessage};
pub use pubsub::{MemoryPubSub, PubSub};
#[cfg(feature = "cable-redis")]
pub use redis_pubsub::RedisPubSub;

/// Build an axum `Router` mounting the cable endpoint (`/cable`) for the given
/// channels over a pub/sub backend (Rails' `mount ActionCable.server`):
///
/// ```ignore
/// let app = doido_cable::cable!(pubsub, [ChatChannel, NotificationsChannel]);
/// ```
#[macro_export]
macro_rules! cable {
    ($pubsub:expr, [ $($channel:expr),* $(,)? ]) => {{
        let mut __registry = $crate::server::ChannelRegistry::new($pubsub);
        $( __registry.register_channel($channel); )*
        $crate::server::route(::std::sync::Arc::new(__registry))
    }};
}
