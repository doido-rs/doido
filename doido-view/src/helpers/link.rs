//! Link helpers (Rails `link_to` / `button_to`).

use super::escape;

/// An anchor: `link_to("Home", "/")` → `<a href="/">Home</a>`.
pub fn link_to(text: &str, url: &str) -> String {
    format!("<a href=\"{}\">{}</a>", escape(url), escape(text))
}

/// An anchor with a `class` attribute.
pub fn link_to_class(text: &str, url: &str, class: &str) -> String {
    format!(
        "<a href=\"{}\" class=\"{}\">{}</a>",
        escape(url),
        escape(class),
        escape(text)
    )
}

/// A button that submits a small form (Rails `button_to`). Methods other than
/// GET/POST are tunneled via a hidden `_method` field.
pub fn button_to(text: &str, url: &str, method: &str) -> String {
    let upper = method.to_uppercase();
    let mut html = format!("<form action=\"{}\" method=\"post\">", escape(url));
    if upper != "POST" && upper != "GET" {
        html.push_str(&format!(
            "<input type=\"hidden\" name=\"_method\" value=\"{upper}\">"
        ));
    }
    html.push_str(&format!(
        "<button type=\"submit\">{}</button></form>",
        escape(text)
    ));
    html
}
