//! The [`Mailer`] trait, implemented by the `#[mailer]` macro.
//!
//! It carries the snake_case mailer name derived from the struct, which drives
//! template resolution following the mailer convention in `docs/08-mailer.md`:
//! `mailers/<mailer_name>/<action>`.

/// A mailer type. The `#[mailer]` attribute generates the implementation from
/// the struct name; the struct's own action methods (which build [`Mail`] values)
/// are left untouched.
///
/// [`Mail`]: crate::Mail
pub trait Mailer {
    /// The snake_case name of this mailer (`UserMailer` → `"user_mailer"`),
    /// derived by the `#[mailer]` macro.
    fn mailer_name() -> &'static str;

    /// Template key for an action, following the mailer template convention:
    /// `mailers/<mailer_name>/<action>`.
    fn template_key(action: &str) -> String {
        format!("mailers/{}/{}", Self::mailer_name(), action)
    }

    /// Build a [`Mail`] by rendering this mailer's templates for `action`:
    /// `mailers/<mailer>/<action>.html.tera` and `.text.tera` (either may be
    /// absent) via [`doido_view`], populating the HTML and text bodies. When
    /// only an HTML part exists, a plain-text fallback is derived from it.
    ///
    /// [`Mail`]: crate::Mail
    fn mail(action: &str, context: &serde_json::Value) -> crate::Mail {
        let key = Self::template_key(action);
        let html = doido_view::render_variant(&key, "html", context).ok();
        let text = doido_view::render_variant(&key, "text", context).ok();
        let text = match (&text, &html) {
            (None, Some(html)) => Some(crate::mailer::html_to_text(html)),
            _ => text,
        };
        crate::Mail {
            body_html: html,
            body_text: text,
            ..Default::default()
        }
    }
}

/// A minimal HTML→plain-text fallback: drop tags, decode a few common entities,
/// and collapse whitespace. Good enough for a text alternative when a mailer
/// ships only an HTML template.
pub fn html_to_text(html: &str) -> String {
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
    let out = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
