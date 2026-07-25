use doido_view::partials::{render_collection, render_partial};
use doido_view::tera_engine::TeraEngine;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

fn write(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn partials_and_collections_render() {
    let dir = TempDir::new().unwrap();
    write(&dir, "_greeting.html.tera", "Hi {{ name }}!");
    write(&dir, "_item.html.tera", "[{{ item }}]");
    doido_view::set_engine(Arc::new(
        TeraEngine::new(dir.path().to_str().unwrap()).unwrap(),
    ));

    // `render_partial("greeting")` renders `_greeting.html.tera`.
    assert_eq!(
        render_partial("greeting", &serde_json::json!({ "name": "Ada" })).unwrap(),
        "Hi Ada!"
    );

    // Collection rendering exposes each item under `as_var` and concatenates.
    let items = [
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
    ];
    assert_eq!(
        render_collection("item", &items, "item").unwrap(),
        "[1][2][3]"
    );
}
