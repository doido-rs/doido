use doido_view::{
    engine::TemplateEngine, renderer::Renderer, response::ViewResponse, tera_engine::TeraEngine,
};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

fn write_tpl(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

struct UppercaseEngine;
impl TemplateEngine for UppercaseEngine {
    fn render(&self, template: &str, _ctx: &serde_json::Value) -> doido_core::Result<String> {
        Ok(template.to_uppercase())
    }
    fn reload(&self) -> doido_core::Result<()> {
        Ok(())
    }
}

#[test]
fn test_full_render_pipeline() {
    let dir = TempDir::new().unwrap();
    write_tpl(
        &dir,
        "posts/show.html.tera",
        "<article>{{ post_title }}</article>",
    );
    write_tpl(
        &dir,
        "layouts/application.html.tera",
        "<!DOCTYPE html><body>{{ content_for_layout }}</body>",
    );
    let engine = Arc::new(TeraEngine::new(dir.path().to_str().unwrap()).unwrap());
    let renderer = Renderer::new(engine, "application");
    let resp = ViewResponse::new("posts/show", serde_json::json!({ "post_title": "Hello" }));
    let html = renderer.render(&resp).unwrap();
    assert_eq!(html, "<!DOCTYPE html><body><article>Hello</article></body>");
}

#[test]
fn test_custom_engine_drop_in() {
    let engine: Arc<dyn TemplateEngine> = Arc::new(UppercaseEngine);
    let renderer = Renderer::new(engine, "");
    let resp = ViewResponse::new("posts/index", serde_json::json!({})).no_layout();
    let html = renderer.render(&resp).unwrap();
    assert_eq!(html, "POSTS/INDEX");
}

#[test]
fn test_status_preserved_in_response() {
    let resp = ViewResponse::new("posts/new", serde_json::json!({})).status(422);
    assert_eq!(resp.status, 422);
}
