use doido_view::engine::TemplateEngine;
use doido_view::tera_engine::TeraEngine;
use std::fs;
use tempfile::TempDir;

fn write_tpl(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn test_tera_engine_renders_template_with_context() {
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "posts/index.html.tera", "<h1>{{ title }}</h1>");
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let ctx = serde_json::json!({ "title": "Hello World" });
    let html = engine.render("posts/index", &ctx).unwrap();
    assert_eq!(html, "<h1>Hello World</h1>");
}

#[test]
fn test_unknown_template_returns_error() {
    let dir = TempDir::new().unwrap();
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let result = engine.render("nonexistent/template", &serde_json::json!({}));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.to_lowercase().contains("template"), "got: {msg}");
}

#[test]
fn test_template_key_resolves_to_html_tera_extension() {
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "posts/index.html.tera", "resolved");
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let result = engine
        .render("posts/index", &serde_json::json!({}))
        .unwrap();
    assert_eq!(result, "resolved");
}

#[test]
fn test_nested_controller_path_resolves_correctly() {
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "admin/users/index.html.tera", "admin-users");
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let result = engine
        .render("admin/users/index", &serde_json::json!({}))
        .unwrap();
    assert_eq!(result, "admin-users");
}

#[test]
fn test_hot_reload_picks_up_template_changes() {
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "posts/index.html.tera", "version1");
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let first = engine
        .render("posts/index", &serde_json::json!({}))
        .unwrap();
    assert_eq!(first, "version1");
    write_tpl(&dir, "posts/index.html.tera", "version2");
    engine.reload().unwrap();
    let second = engine
        .render("posts/index", &serde_json::json!({}))
        .unwrap();
    assert_eq!(second, "version2");
}
