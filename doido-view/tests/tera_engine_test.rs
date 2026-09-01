//! Tera engine integration tests. Framework template registration mutates process-global
//! state; serialise those tests so parallel runs don't race the registry.

use doido_view::engine::TemplateEngine;
use doido_view::tera_engine::TeraEngine;
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static FRAMEWORK_TEMPLATE_LOCK: Mutex<()> = Mutex::new(());

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

#[test]
fn render_named_resolves_exact_template_path() {
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "mailers/welcome.text.tera", "Hello {{ name }}");
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let out = engine
        .render_named(
            "mailers/welcome.text.tera",
            &serde_json::json!({ "name": "Ada" }),
        )
        .unwrap();
    assert_eq!(out, "Hello Ada");
}

#[test]
fn framework_template_renders_without_app_file() {
    let _lock = FRAMEWORK_TEMPLATE_LOCK.lock().unwrap();
    // Registered globally; unique standalone name avoids interfering with other
    // tests that share this process's framework-template registry.
    doido_view::register_framework_template(
        "fwtest/builtin_only.html.tera",
        "<p>{{ who }} from framework</p>",
    );
    let dir = TempDir::new().unwrap();
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let html = engine
        .render("fwtest/builtin_only", &serde_json::json!({ "who": "hi" }))
        .unwrap();
    assert_eq!(html, "<p>hi from framework</p>");
}

#[test]
fn app_template_overrides_framework_template() {
    let _lock = FRAMEWORK_TEMPLATE_LOCK.lock().unwrap();
    doido_view::register_framework_template("fwtest/overridable.html.tera", "framework-version");
    let dir = TempDir::new().unwrap();
    write_tpl(&dir, "fwtest/overridable.html.tera", "app-version");
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let html = engine
        .render("fwtest/overridable", &serde_json::json!({}))
        .unwrap();
    assert_eq!(
        html, "app-version",
        "app template must override framework one"
    );
}

#[test]
fn framework_view_inheritance_resolves_in_single_load() {
    let _lock = FRAMEWORK_TEMPLATE_LOCK.lock().unwrap();
    // A framework view that `extends` another template must resolve inheritance
    // in the single-pass load. Register the parent as a framework template too so
    // this test never poisons the process-shared registry for other tests (the
    // registry has no removal; an unsatisfiable framework template would trip the
    // resilient app-only fallback everywhere else).
    doido_view::register_framework_template(
        "fwtest/layout.html.tera",
        "<main>{% block content %}{% endblock content %}</main>",
    );
    doido_view::register_framework_template(
        "fwtest/extends_layout.html.tera",
        "{% extends \"fwtest/layout.html.tera\" %}{% block content %}framed{% endblock content %}",
    );
    let dir = TempDir::new().unwrap();
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let html = engine
        .render("fwtest/extends_layout", &serde_json::json!({}))
        .unwrap();
    assert_eq!(html, "<main>framed</main>");
}

#[test]
fn app_layout_satisfies_framework_view_inheritance() {
    let _lock = FRAMEWORK_TEMPLATE_LOCK.lock().unwrap();
    // The production case: a framework view extends a layout the *app* provides.
    // Registered globally with the app layout present here; the layout name is
    // unique so it doesn't collide with other tests.
    doido_view::register_framework_template(
        "fwtest/uses_app_layout.html.tera",
        "{% extends \"fwtest/applayout.html.tera\" %}{% block content %}from-app-layout{% endblock content %}",
    );
    // Also register the layout as a framework template so other tests (which lack
    // this app file) still find the parent and don't hit the app-only fallback.
    doido_view::register_framework_template(
        "fwtest/applayout.html.tera",
        "<section>{% block content %}{% endblock content %}</section>",
    );
    let dir = TempDir::new().unwrap();
    // App provides its own layout of the same name — it must win over the framework one.
    write_tpl(
        &dir,
        "fwtest/applayout.html.tera",
        "<app-layout>{% block content %}{% endblock content %}</app-layout>",
    );
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let html = engine
        .render("fwtest/uses_app_layout", &serde_json::json!({}))
        .unwrap();
    assert_eq!(html, "<app-layout>from-app-layout</app-layout>");
}

#[test]
fn template_inheritance_renders_layout() {
    let dir = TempDir::new().unwrap();
    write_tpl(
        &dir,
        "layouts/base.html.tera",
        "<html>{% block content %}{% endblock content %}</html>",
    );
    write_tpl(
        &dir,
        "pages/show.html.tera",
        "{% extends \"layouts/base.html.tera\" %}{% block content %}body{% endblock content %}",
    );
    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let html = engine.render("pages/show", &serde_json::json!({})).unwrap();
    assert!(html.contains("body"));
}
