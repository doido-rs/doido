//! Action View-style HTML helpers (forms, links, assets, tags, formatting…).

pub mod form;
pub mod link;

/// Minimal HTML text/attribute escaping, shared by the helpers.
pub(crate) fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
