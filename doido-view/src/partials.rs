//! Partial and collection rendering (Rails `render "posts/form"` /
//! `render partial:, collection:`).
//!
//! A partial's file is the `_`-prefixed name in its directory, e.g.
//! `render_partial("posts/form")` renders `posts/_form.html.tera`.

use crate::render;
use doido_core::Result;
use serde_json::{json, Value};

/// Render a single partial with `context`.
pub fn render_partial(name: &str, context: &Value) -> Result<String> {
    render(&partial_path(name), context)
}

/// Render `partial` once per item, exposing each item under `as_var`, and
/// concatenate the results (Rails collection rendering).
pub fn render_collection(partial: &str, items: &[Value], as_var: &str) -> Result<String> {
    let mut out = String::new();
    for item in items {
        let ctx = json!({ as_var: item });
        out.push_str(&render_partial(partial, &ctx)?);
    }
    Ok(out)
}

/// `posts/form` → `posts/_form`; `form` → `_form`.
fn partial_path(name: &str) -> String {
    match name.rsplit_once('/') {
        Some((dir, base)) => format!("{dir}/_{base}"),
        None => format!("_{name}"),
    }
}
