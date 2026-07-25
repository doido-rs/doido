//! Form helpers (Rails `form_with` + field helpers).

use super::escape;

/// Open a `<form>`. Methods other than GET/POST are tunneled through a hidden
/// `_method` field (Rails), and a CSRF `authenticity_token` field is included
/// when a token is supplied.
pub fn form_tag(action: &str, method: &str, csrf_token: Option<&str>) -> String {
    let upper = method.to_uppercase();
    let (html_method, override_method) = match upper.as_str() {
        "GET" => ("get", None),
        "POST" => ("post", None),
        other => ("post", Some(other.to_string())),
    };
    let mut html = format!(
        "<form action=\"{}\" method=\"{}\">",
        escape(action),
        html_method
    );
    if let Some(m) = override_method {
        html.push_str(&format!(
            "<input type=\"hidden\" name=\"_method\" value=\"{m}\">"
        ));
    }
    if let Some(token) = csrf_token {
        html.push_str(&format!(
            "<input type=\"hidden\" name=\"authenticity_token\" value=\"{}\">",
            escape(token)
        ));
    }
    html
}

/// Close a form.
pub fn form_end() -> &'static str {
    "</form>"
}

/// A text input.
pub fn text_field(name: &str, value: &str) -> String {
    format!(
        "<input type=\"text\" name=\"{}\" value=\"{}\">",
        escape(name),
        escape(value)
    )
}

/// A textarea.
pub fn text_area(name: &str, value: &str) -> String {
    format!(
        "<textarea name=\"{}\">{}</textarea>",
        escape(name),
        escape(value)
    )
}

/// A hidden input.
pub fn hidden_field(name: &str, value: &str) -> String {
    format!(
        "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
        escape(name),
        escape(value)
    )
}

/// A label for a field.
pub fn label(field: &str, text: &str) -> String {
    format!("<label for=\"{}\">{}</label>", escape(field), escape(text))
}

/// A submit button.
pub fn submit(label: &str) -> String {
    format!("<input type=\"submit\" value=\"{}\">", escape(label))
}
