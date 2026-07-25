//! Stream-name helpers (Rails `stream_from` / `stream_for`).

/// A named stream: `stream_from("chat_room_1")` → `"chat_room_1"`.
pub fn stream_from(name: &str) -> String {
    name.to_string()
}

/// A per-record stream: `stream_for("Room", 7)` → `"Room:7"`.
pub fn stream_for(record_type: &str, id: impl std::fmt::Display) -> String {
    format!("{record_type}:{id}")
}
