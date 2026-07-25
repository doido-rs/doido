//! Generic tag builders (Rails `tag` / `content_tag`).

use super::escape;

fn render_attrs(attrs: &[(&str, &str)]) -> String {
    attrs
        .iter()
        .map(|(k, v)| format!(" {}=\"{}\"", k, escape(v)))
        .collect()
}

/// A standalone (void) tag: `tag("br", &[])` → `<br>`;
/// `tag("input", &[("type","text")])` → `<input type="text">`.
pub fn tag(name: &str, attrs: &[(&str, &str)]) -> String {
    format!("<{}{}>", name, render_attrs(attrs))
}

/// A tag wrapping (escaped) content:
/// `content_tag("p", "hi", &[("class","lead")])` → `<p class="lead">hi</p>`.
pub fn content_tag(name: &str, content: &str, attrs: &[(&str, &str)]) -> String {
    format!(
        "<{0}{1}>{2}</{0}>",
        name,
        render_attrs(attrs),
        escape(content)
    )
}
