use doido_generators::generators::new::ProjectGenerator;
use doido_generators::{default_registry, Generator};

#[test]
fn test_new_generates_all_expected_files() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"my-app/Cargo.toml"));
    assert!(paths.contains(&"my-app/src/main.rs"));
    assert!(paths.contains(&"my-app/config/application.toml"));
    assert!(paths.contains(&"my-app/config/routes.rs"));
    assert!(paths.contains(&"my-app/src/controllers/.gitkeep"));
    assert!(paths.contains(&"my-app/src/models/.gitkeep"));
    assert!(paths.contains(&"my-app/views/layouts/application.html.tera"));
    assert!(paths.contains(&"my-app/db/migrations/.gitkeep"));
    assert!(paths.contains(&"my-app/tests/integration_test.rs"));
    assert!(paths.contains(&"my-app/.gitignore"));
}

#[test]
fn test_new_sqlite_cargo_toml_has_sqlite_feature() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    let cargo_toml = files
        .iter()
        .find(|f| f.path == "my-app/Cargo.toml")
        .unwrap();
    assert!(cargo_toml.content.contains("my-app"));
    assert!(cargo_toml.content.contains("sqlite"));
}

#[test]
fn test_new_postgres_sets_correct_database_url() {
    let files = ProjectGenerator
        .generate(&["blog", "--database=postgres"])
        .unwrap();
    let app_config = files
        .iter()
        .find(|f| f.path == "blog/config/application.toml")
        .unwrap();
    assert!(app_config
        .content
        .contains("postgres://localhost/blog_development"));
}

#[test]
fn test_new_mysql_sets_correct_database_url() {
    let files = ProjectGenerator
        .generate(&["store", "--database=mysql"])
        .unwrap();
    let app_config = files
        .iter()
        .find(|f| f.path == "store/config/application.toml")
        .unwrap();
    assert!(app_config
        .content
        .contains("mysql://localhost/store_development"));
}

#[test]
fn test_new_sqlite_default_when_no_database_flag() {
    let files = ProjectGenerator.generate(&["my-app"]).unwrap();
    let app_config = files
        .iter()
        .find(|f| f.path == "my-app/config/application.toml")
        .unwrap();
    assert!(app_config.content.contains("sqlite://db/development.db"));
}

#[test]
fn test_new_integration_test_file_has_passing_stub() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    let test_file = files
        .iter()
        .find(|f| f.path == "my-app/tests/integration_test.rs")
        .unwrap();
    assert!(test_file.content.contains("#[test]"));
    assert!(test_file.content.contains("assert!(true)"));
}

#[test]
fn test_new_output_is_deterministic() {
    let files1 = ProjectGenerator
        .generate(&["app1", "--database=sqlite"])
        .unwrap();
    let files2 = ProjectGenerator
        .generate(&["app1", "--database=sqlite"])
        .unwrap();
    let paths1: Vec<&str> = files1.iter().map(|f| f.path.as_str()).collect();
    let paths2: Vec<&str> = files2.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths1, paths2);
    assert_eq!(files1[0].content, files2[0].content);
}

#[test]
fn test_new_requires_name_argument() {
    let result = ProjectGenerator.generate(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("name"));
}

#[test]
fn test_new_rejects_unknown_database() {
    let result = ProjectGenerator.generate(&["my-app", "--database=oracle"]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("oracle"));
}

#[test]
fn test_new_registered_in_default_registry() {
    let registry = default_registry();
    let files = registry
        .run("new", &["my-app", "--database=sqlite"])
        .unwrap();
    assert!(!files.is_empty());
}
