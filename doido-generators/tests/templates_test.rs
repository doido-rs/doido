use doido_generators::templates;
use std::fs;
use tempfile::TempDir;

#[test]
fn get_returns_builtin_controller_template() {
    let content = templates::get("controller/controller.rs.template");
    assert!(content.contains("{snake}"));
    assert!(content.contains("#[controller]"));
}

#[test]
fn get_with_root_prefers_project_override() {
    let dir = TempDir::new().unwrap();
    let rel = "controller/controller.rs.template";
    let root = dir.path().join("templates");
    let override_path = root.join(rel);
    fs::create_dir_all(override_path.parent().unwrap()).unwrap();
    fs::write(&override_path, "override template").unwrap();
    assert_eq!(templates::get_with_root(&root, rel), "override template");
}

#[test]
fn builtin_templates_lists_known_paths() {
    let names: Vec<_> = templates::builtin_templates()
        .iter()
        .map(|(rel, _)| *rel)
        .collect();
    assert!(names.contains(&"migration/migration.rs.template"));
    assert!(names.contains(&"models/model.rs.template"));
}

#[test]
fn project_root_points_at_templates_directory() {
    assert_eq!(
        templates::project_root(),
        std::path::PathBuf::from("templates")
    );
}
