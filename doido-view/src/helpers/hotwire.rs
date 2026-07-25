//! Hotwire helpers (Turbo Frames/Streams + Stimulus).

/// A Turbo Frame wrapping `content`.
pub fn turbo_frame(id: &str, content: &str) -> String {
    format!("<turbo-frame id=\"{id}\">{content}</turbo-frame>")
}

/// A Turbo Stream action (`append`/`replace`/`remove`/…) targeting `target`.
pub fn turbo_stream(action: &str, target: &str, content: &str) -> String {
    format!(
        "<turbo-stream action=\"{action}\" target=\"{target}\"><template>{content}</template></turbo-stream>"
    )
}

/// A Stimulus `data-controller` attribute string.
pub fn stimulus_controller(name: &str) -> String {
    format!("data-controller=\"{name}\"")
}
