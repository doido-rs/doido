//! Sanitization helpers (Rails `sanitize` / `strip_tags` / `h`).

/// Escape HTML special characters (Rails `h`).
pub fn html_escape(s: &str) -> String {
    super::escape(s)
}

/// Remove all HTML tags, keeping the text between them (Rails `strip_tags`).
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

/// A conservative sanitize: strip every tag, then escape the remaining text so
/// no markup can survive (Rails `sanitize`, allowlist not yet supported).
pub fn sanitize(html: &str) -> String {
    html_escape(&strip_tags(html))
}
