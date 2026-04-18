use doido_view::renderer::Renderer;
use doido_view::response::ViewResponse;
use doido_view::tera_engine::TeraEngine;
use std::sync::Arc;
use tempfile::TempDir;
use std::fs;

fn write_tpl(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn test_renderer_uses_default_layout() {
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "posts/index.html.tera", "<main>content</main>");
    write_tpl(&dir, "layouts/application.html.tera", "<html>{{ content_for_layout }}</html>");
    let engine = Arc::new(TeraEngine::new(dir.path().to_str().unwrap()).unwrap());
    let renderer = Renderer::new(engine, "application");
    let resp = ViewResponse::new("posts/index", serde_json::json!({}));
    let html = renderer.render(&resp).unwrap();
    assert_eq!(html, "<html><main>content</main></html>");
}

#[test]
fn test_renderer_no_layout_skips_layout() {
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "posts/index.html.tera", "<main>bare</main>");
    let engine = Arc::new(TeraEngine::new(dir.path().to_str().unwrap()).unwrap());
    let renderer = Renderer::new(engine, "application");
    let resp = ViewResponse::new("posts/index", serde_json::json!({})).no_layout();
    let html = renderer.render(&resp).unwrap();
    assert_eq!(html, "<main>bare</main>");
}

#[test]
fn test_renderer_custom_layout_override() {
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "posts/index.html.tera", "body");
    write_tpl(&dir, "layouts/admin.html.tera", "<admin>{{ content_for_layout }}</admin>");
    let engine = Arc::new(TeraEngine::new(dir.path().to_str().unwrap()).unwrap());
    let renderer = Renderer::new(engine, "application");
    let resp = ViewResponse::new("posts/index", serde_json::json!({})).layout("admin");
    let html = renderer.render(&resp).unwrap();
    assert_eq!(html, "<admin>body</admin>");
}
