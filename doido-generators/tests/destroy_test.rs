use doido_generators::commands::destroy::destroyable_paths;
use doido_generators::GeneratedFile;

fn f(path: &str) -> GeneratedFile {
    GeneratedFile {
        path: path.to_string(),
        content: String::new(),
    }
}

#[test]
fn destroyable_paths_excludes_shared_files() {
    let files = vec![
        f("app/models/widget.rs"),
        f("app/models/mod.rs"),
        f("tests/widget_test.rs"),
        f("config/routes.rs"),
        f("db/migration/m1_create.rs"),
        f("db/migration/mod.rs"),
    ];
    let d = destroyable_paths(&files);
    assert!(d.contains(&"app/models/widget.rs".to_string()));
    assert!(d.contains(&"tests/widget_test.rs".to_string()));
    assert!(d.contains(&"db/migration/m1_create.rs".to_string()));
    assert!(!d.iter().any(|p| p.ends_with("mod.rs")));
    assert!(!d.iter().any(|p| p.ends_with("routes.rs")));
}
