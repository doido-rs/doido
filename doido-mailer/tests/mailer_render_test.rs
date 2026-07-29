//! `Mailer::mail(action, ctx)` renders the mailer's html/text templates via
//! `doido_view` and populates the bodies, with an HTML→text fallback.

use doido_mailer::mailer::html_to_text;
use doido_mailer::{mailer, Mailer};
use doido_view::engine::TemplateEngine;
use std::sync::Arc;

// A stub engine: renders any `.html.tera` name, but has no `.text.tera` (errors),
// so the mailer must fall back to deriving text from the HTML.
struct HtmlOnlyEngine;
impl TemplateEngine for HtmlOnlyEngine {
    fn render(&self, template: &str, _ctx: &serde_json::Value) -> doido_core::Result<String> {
        if template.ends_with(".text.tera") {
            Err(doido_core::anyhow::anyhow!("no text template"))
        } else {
            Ok(format!("<h1>Hello from {template}</h1>"))
        }
    }
    fn reload(&self) -> doido_core::Result<()> {
        Ok(())
    }
}

#[mailer]
struct UserMailer;

#[test]
fn html_to_text_strips_tags_and_collapses_whitespace() {
    assert_eq!(html_to_text("<p>Hi   there</p>"), "Hi there");
    assert_eq!(html_to_text("a &amp; b &lt;c&gt;"), "a & b <c>");
}

#[test]
fn mail_renders_html_and_falls_back_to_text() {
    doido_view::set_engine(Arc::new(HtmlOnlyEngine));

    let mail = UserMailer::mail("welcome", &serde_json::json!({ "name": "Ada" }));

    let html = mail.body_html.expect("html body rendered");
    assert!(html.contains("mailers/user_mailer/welcome.html.tera"));
    // No text template → text derived from the HTML (tags stripped).
    let text = mail.body_text.expect("text body derived from html");
    assert_eq!(text, "Hello from mailers/user_mailer/welcome.html.tera");
    assert!(!text.contains('<'), "fallback text has no tags");
}
