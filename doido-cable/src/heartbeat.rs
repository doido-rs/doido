//! ActionCable heartbeat pings. Apps drive a periodic timer (e.g. a tokio
//! interval) and send [`ping_now`] frames to keep connections alive.

use crate::protocol::ServerFrame;
use std::time::{SystemTime, UNIX_EPOCH};

/// A heartbeat ping frame carrying `epoch_secs`.
pub fn ping(epoch_secs: i64) -> ServerFrame {
    ServerFrame::Ping {
        message: epoch_secs,
    }
}

/// A heartbeat ping for the current unix time.
pub fn ping_now() -> ServerFrame {
    ping(unix_now())
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
