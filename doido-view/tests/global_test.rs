//! Process-global template engine (`OnceLock` — one test binary).

use doido_view::engine::TemplateEngine;
use doido_view::{init, render, render_variant, set_engine, try_engine};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

struct StubEngine;
impl TemplateEngine for StubEngine {
    fn render(&self, template: &str, _ctx: &serde_json::Value) -> doido_core::Result<String> {
        Ok(format!("stub:{template}"))
    }
    fn render_named(&self, name: &str, _ctx: &serde_json::Value) -> doido_core::Result<String> {
        Ok(format!("named:{name}"))
    }
    fn reload(&self) -> doido_core::Result<()> {
        Ok(())
    }
}

#[test]
fn global_engine_lifecycle() {
    assert!(try_engine().is_none());
    assert!(render("any", &serde_json::json!({})).is_err());

    set_engine(Arc::new(StubEngine));
    assert!(try_engine().is_some());
    assert_eq!(
        render("posts/index", &serde_json::json!({})).unwrap(),
        "stub:posts/index"
    );
    assert_eq!(
        render_variant("mailers/welcome", "text", &serde_json::json!({})).unwrap(),
        "named:mailers/welcome.text.tera"
    );

    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("home.html.tera"), "<p>{{ name }}</p>").unwrap();
    init(dir.path().to_str().unwrap()).unwrap();
    // Second init is ignored — still the stub from set_engine.
    assert_eq!(
        render("posts/index", &serde_json::json!({})).unwrap(),
        "stub:posts/index"
    );
}
