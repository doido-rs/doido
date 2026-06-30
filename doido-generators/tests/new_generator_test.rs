use doido_generators::generators::new::ProjectGenerator;
use doido_generators::{default_registry, Generator, TEMPLATE_WORKSPACE_PATH};

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
    assert!(paths.contains(&"my-app/config/development.yml"));
    assert!(paths.contains(&"my-app/config/test.yml"));
    assert!(paths.contains(&"my-app/config/production.yml"));
    assert!(paths.contains(&"my-app/app/controllers/hello_controller.rs"));
    assert!(paths.contains(&"my-app/app/controllers/mod.rs"));
    assert!(paths.contains(&"my-app/app/models/.gitkeep"));
    // `doido db generate entity` writes SeaORM entities here by default.
    assert!(paths.contains(&"my-app/app/models/_entities/.gitkeep"));
    assert!(paths.contains(&"my-app/app/views/layouts/application.html.tera"));
    assert!(paths.contains(&"my-app/db/schema/.gitkeep"));
    // `db/migration` is a SeaORM migration project, not an empty placeholder.
    assert!(paths.contains(&"my-app/db/migration/Cargo.toml"));
    assert!(paths.contains(&"my-app/db/migration/src/lib.rs"));
    assert!(paths.contains(&"my-app/db/migration/src/main.rs"));
    assert!(paths.contains(&"my-app/tests/integration_test.rs"));
    assert!(paths.contains(&"my-app/.gitignore"));
    assert!(paths.contains(&"my-app/README.md"));
}

#[test]
fn test_new_readme_is_titled_with_the_app_name() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    let readme = files
        .iter()
        .find(|f| f.path == "my-app/README.md")
        .unwrap();
    // The `{doido_name}` placeholder must be substituted, not left raw.
    assert!(readme.content.contains("# my-app"));
    assert!(!readme.content.contains("{doido_name}"));
}

#[test]
fn test_new_template_includes_json_hello_action() {
    let files = ProjectGenerator
        .generate(&["api", "--database=sqlite"])
        .unwrap();
    let hello = files
        .iter()
        .find(|f| f.path == "api/app/controllers/hello_controller.rs")
        .unwrap();
    assert!(hello.content.contains("Hello word!"));
    assert!(hello.content.contains("json!("));
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
    assert!(cargo_toml.content.contains("serde_json"));
    assert!(cargo_toml.content.contains("axum"));
    assert!(
        cargo_toml.content.contains(&format!(
            "doido = {{ path = \"{}/doido\" }}",
            TEMPLATE_WORKSPACE_PATH
        )),
        "generated Cargo.toml must point `doido` at the local workspace crate"
    );
    assert!(
        cargo_toml.content.contains(&format!(
            "doido-controller = {{ path = \"{}/doido-controller\" }}",
            TEMPLATE_WORKSPACE_PATH
        )),
        "generated Cargo.toml must point `doido-controller` at the local workspace crate"
    );
}

#[test]
fn test_new_migration_crate_uses_selected_backend() {
    let files = ProjectGenerator
        .generate(&["blog", "--database=postgres"])
        .unwrap();
    let migration_cargo = files
        .iter()
        .find(|f| f.path == "blog/db/migration/Cargo.toml")
        .unwrap();
    assert!(migration_cargo.content.contains("sea-orm-migration"));
    assert!(migration_cargo.content.contains("sqlx-postgres"));
}

#[test]
fn test_new_env_yml_files_carry_per_env_database_url() {
    let files = ProjectGenerator
        .generate(&["blog", "--database=postgres"])
        .unwrap();
    let find = |path: &str| {
        files
            .iter()
            .find(|f| f.path == path)
            .unwrap_or_else(|| panic!("missing {path}"))
            .content
            .clone()
    };
    // Dev/test carry working local credentials, host and port.
    assert!(find("blog/config/development.yml")
        .contains("postgres://postgres:postgres@localhost:5432/blog_development"));
    assert!(find("blog/config/test.yml")
        .contains("postgres://postgres:postgres@localhost:5432/blog_test"));
    // Production keeps the same shape but never ships a real password.
    let prod = find("blog/config/production.yml");
    assert!(prod.contains("postgres://postgres:CHANGE_ME@localhost:5432/blog_production"));
    assert!(!prod.contains(":postgres@"));
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
        .contains("postgres://postgres:postgres@localhost:5432/blog_development"));
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
        .contains("mysql://root:password@localhost:3306/store_development"));
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
