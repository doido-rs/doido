use doido_generators::generators::new::ProjectGenerator;
use doido_generators::{default_registry, DependencyMode, Generator, DOIDO_VERSION};

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
    assert!(paths.contains(&"my-app/app/helpers/mod.rs"));
    assert!(paths.contains(&"my-app/app/helpers/application_helper.rs"));
    assert!(paths.contains(&"my-app/app/generators/mod.rs"));
    assert!(paths.contains(&"my-app/app/models/.gitkeep"));
    // `doido db generate entity` writes SeaORM entities here by default.
    assert!(paths.contains(&"my-app/app/models/_entities/.gitkeep"));
    assert!(paths.contains(&"my-app/app/models/_entities/mod.rs"));
    assert!(paths.contains(&"my-app/app/views/layouts/application.html.tera"));
    assert!(paths.contains(&"my-app/db/schema/.gitkeep"));
    // Migrations compile into the app binary as `db/migration/mod.rs` + `m*.rs`.
    assert!(paths.contains(&"my-app/db/migration/mod.rs"));
    assert!(!paths.contains(&"my-app/db/migration/Cargo.toml"));
    assert!(paths.contains(&"my-app/db/seeds.rs"));
    assert!(!paths.contains(&"my-app/db/seed/Cargo.toml"));
    assert!(paths.contains(&"my-app/tests/integration_test.rs"));
    let main_rs = files
        .iter()
        .find(|f| f.path == "my-app/src/main.rs")
        .unwrap();
    assert!(main_rs.content.contains("mod helpers;"));
    // The app CLI is the Doido builder, with a marker for app generators.
    assert!(main_rs.content.contains("mod generators;"));
    assert!(main_rs.content.contains("doido::Doido::new()"));
    assert!(main_rs.content.contains("@generated-generators"));
    assert!(paths.contains(&"my-app/.gitignore"));
    assert!(paths.contains(&"my-app/README.md"));
    assert!(paths.contains(&"my-app/mise.toml"));
    assert!(paths.contains(&"my-app/.cargo/config.toml"));
}

#[test]
fn test_new_mise_toml_pins_the_rust_toolchain() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    let mise = files.iter().find(|f| f.path == "my-app/mise.toml").unwrap();
    assert!(mise.content.contains("[tools]"));
    assert!(mise.content.contains("rust ="));
}

#[test]
fn test_new_readme_is_titled_with_the_app_name() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    let readme = files.iter().find(|f| f.path == "my-app/README.md").unwrap();
    // The `{doido_name}` placeholder must be substituted, not left raw.
    assert!(readme.content.contains("# my-app"));
    assert!(!readme.content.contains("{doido_name}"));
}

#[test]
fn test_new_without_cable_flag_has_no_channels() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    assert!(!files
        .iter()
        .any(|f| f.path.starts_with("my-app/app/channels/")));
    let cargo = files
        .iter()
        .find(|f| f.path == "my-app/Cargo.toml")
        .unwrap();
    assert!(!cargo.content.contains("doido-cable"));
}

#[test]
fn test_new_cable_flag_adds_example_channel() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite", "--cable"])
        .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"my-app/app/channels/mod.rs"));
    assert!(paths.contains(&"my-app/app/channels/chat_channel.rs"));
}

#[test]
fn test_new_cable_flag_wires_dependency_and_module() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite", "--cable"])
        .unwrap();
    let find = |path: &str| {
        files
            .iter()
            .find(|f| f.path == path)
            .unwrap_or_else(|| panic!("missing {path}"))
            .content
            .clone()
    };
    // Dependency wiring: doido-cable + async-trait land in Cargo.toml.
    let cargo = find("my-app/Cargo.toml");
    assert!(cargo.contains("doido-cable ="));
    assert!(cargo.contains("async-trait ="));
    // Module wiring: main.rs pulls in app/channels.
    let main_rs = find("my-app/src/main.rs");
    assert!(main_rs.contains("mod channels;"));
    // Setup docs land in the README.
    assert!(find("my-app/README.md").contains("doido-cable"));
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
    assert!(hello.content.contains("ApplicationHelper::greet"));
    assert!(hello.content.contains("doido::controller::"));
    assert!(!hello.content.contains("doido_controller::"));
}

#[test]
fn test_new_cargo_config_aliases_doido_to_app_binary() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    let cargo_toml = files
        .iter()
        .find(|f| f.path == "my-app/Cargo.toml")
        .unwrap();
    assert!(
        cargo_toml.content.contains("default-run = \"my-app\""),
        "generated Cargo.toml must default to the app binary"
    );

    let cargo_config = files
        .iter()
        .find(|f| f.path == "my-app/.cargo/config.toml")
        .unwrap();
    assert!(
        cargo_config
            .content
            .contains("doido = \"run --bin my-app --\""),
        "cargo doido must delegate to the app binary"
    );
    cargo_config
        .content
        .parse::<toml::Table>()
        .expect("valid .cargo/config.toml");
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
    assert!(cargo_toml.content.contains("doido-model ="));
    assert!(cargo_toml.content.contains("features = [\"sqlite\"]"));
    assert!(cargo_toml.content.contains("serde_json"));
    assert!(
        cargo_toml.content.contains("doido-controller ="),
        "generated app must depend on doido-controller (axum is re-exported there)"
    );
    assert!(
        !cargo_toml.content.contains("\naxum ="),
        "generated app must not depend on axum directly"
    );
    let mode = DependencyMode::resolve();
    if mode.use_path {
        assert!(
            cargo_toml
                .content
                .contains(&format!("path = \"{}/doido\"", mode.workspace_path)),
            "generated Cargo.toml must point `doido` at the local workspace crate"
        );
        assert!(
            cargo_toml.content.contains(&format!(
                "path = \"{}/doido-controller\"",
                mode.workspace_path
            )),
            "generated Cargo.toml must point `doido-controller` at the local workspace crate"
        );
    } else {
        assert!(
            !cargo_toml.content.contains("path ="),
            "isolated/published binary must not emit path dependencies"
        );
        assert!(
            cargo_toml.content.contains(DOIDO_VERSION),
            "isolated/published binary must pin crates.io version"
        );
    }
}

#[test]
fn test_new_controller_dep_shares_database_feature() {
    let files = ProjectGenerator
        .generate(&["app", "--database=postgres"])
        .unwrap();
    let cargo = files.iter().find(|f| f.path == "app/Cargo.toml").unwrap();
    assert!(
        cargo.content.contains("doido-controller = {")
            && cargo.content.contains("features = [\"postgres\"]")
    );
}

#[test]
fn test_new_app_cargo_toml_doido_model_feature_matches_database() {
    let cases = [
        ("sqlite", "features = [\"sqlite\"]"),
        ("postgres", "features = [\"postgres\"]"),
        ("mysql", "features = [\"mysql\"]"),
    ];
    for (database, model_feature) in cases {
        let files = ProjectGenerator
            .generate(&["app", &format!("--database={database}")])
            .unwrap();
        let cargo = files
            .iter()
            .find(|f| f.path == "app/Cargo.toml")
            .unwrap_or_else(|| panic!("missing app Cargo.toml for {database}"));
        assert!(
            cargo.content.contains("doido-model ="),
            "{database}: app must depend on doido-model"
        );
        assert!(
            cargo.content.contains(model_feature),
            "{database}: app doido-model must declare {model_feature}"
        );
    }
}

#[test]
fn test_new_migration_module_uses_selected_backend() {
    let files = ProjectGenerator
        .generate(&["blog", "--database=postgres"])
        .unwrap();
    let migration_mod = files
        .iter()
        .find(|f| f.path == "blog/db/migration/mod.rs")
        .unwrap();
    assert!(
        migration_mod
            .content
            .contains("doido::model::sea_orm_migration"),
        "migration mod.rs must import sea_orm_migration via doido::model"
    );
    let cargo = files.iter().find(|f| f.path == "blog/Cargo.toml").unwrap();
    assert!(
        cargo.content.contains("features = [\"postgres\"]"),
        "app doido dependency must enable the postgres backend"
    );
    assert!(
        !cargo.content.contains("migration = { path"),
        "app must not depend on a separate migration crate"
    );
}

#[test]
fn test_new_seeds_module_replaces_the_seed_crate() {
    let files = ProjectGenerator
        .generate(&["blog", "--database=postgres"])
        .unwrap();
    // The seeder is now an app module compiled into the app binary, not a crate.
    let seeds = files
        .iter()
        .find(|f| f.path == "blog/db/seeds.rs")
        .expect("db/seeds.rs must be generated");
    assert!(
        seeds
            .content
            .contains("pub async fn run(db: &DatabaseConnection)"),
        "db/seeds.rs must expose `pub async fn run(db: &DatabaseConnection)`"
    );
    assert!(
        !files.iter().any(|f| f.path.contains("db/seed/")),
        "the db/seed crate must no longer be generated"
    );
    let main_rs = files.iter().find(|f| f.path == "blog/src/main.rs").unwrap();
    assert!(
        main_rs.content.contains("mod seed;") && main_rs.content.contains(".seeder(seed::run)"),
        "src/main.rs must wire and register the seeder"
    );
    assert!(
        main_rs.content.contains("mod migration;")
            && main_rs
                .content
                .contains(".migrator::<migration::Migrator>()"),
        "src/main.rs must wire and register the migrator"
    );
    let cargo = files.iter().find(|f| f.path == "blog/Cargo.toml").unwrap();
    assert!(
        !cargo.content.contains("[workspace]"),
        "generated app must not declare a workspace for db/migration"
    );
    assert!(
        !cargo.content.contains("migration = { path"),
        "app must not depend on a separate migration crate"
    );
    assert!(
        cargo.content.contains("async-trait = \"0.1\""),
        "app Cargo.toml must declare async-trait for MigratorTrait"
    );
}

#[test]
fn test_new_migration_module_compiles_with_app_doido_features() {
    let cases = [
        ("sqlite", "features = [\"sqlite\""),
        ("postgres", "features = [\"postgres\""),
        ("mysql", "features = [\"mysql\""),
    ];
    for (database, feature) in cases {
        let files = ProjectGenerator
            .generate(&["app", &format!("--database={database}")])
            .unwrap();
        let migration_mod = files
            .iter()
            .find(|f| f.path == "app/db/migration/mod.rs")
            .unwrap_or_else(|| panic!("missing db/migration/mod.rs for {database}"));
        assert!(
            migration_mod
                .content
                .contains("doido::model::sea_orm_migration"),
            "{database}: migration mod.rs must import via doido::model"
        );
        let cargo = files.iter().find(|f| f.path == "app/Cargo.toml").unwrap();
        assert!(
            cargo.content.contains(feature),
            "{database}: app doido line must include {feature}"
        );
        assert!(
            !cargo.content.contains("sea-orm-migration"),
            "{database}: app must not declare sea-orm-migration directly"
        );
    }
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
fn test_new_api_marks_project_api_only() {
    let files = ProjectGenerator.generate(&["blog", "--api"]).unwrap();
    let app_config = files
        .iter()
        .find(|f| f.path == "blog/config/application.toml")
        .unwrap();
    assert!(
        app_config.content.contains("api_only = true"),
        "--api must write the api_only marker under [app]:\n{}",
        app_config.content
    );
}

#[test]
fn test_new_without_api_omits_marker() {
    let files = ProjectGenerator.generate(&["blog"]).unwrap();
    let app_config = files
        .iter()
        .find(|f| f.path == "blog/config/application.toml")
        .unwrap();
    assert!(!app_config.content.contains("api_only"));
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
        .run(
            "new",
            &[
                "my-app",
                "--database=sqlite",
                "--cache=memory",
                "--jobs=memory",
            ],
        )
        .unwrap();
    assert!(!files.is_empty());
}

#[test]
fn test_new_includes_storage_bootstrap_migration() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite"])
        .unwrap();
    let mod_rs = files
        .iter()
        .find(|f| f.path == "my-app/db/migration/mod.rs")
        .unwrap();
    assert!(mod_rs
        .content
        .contains("m20260101000000_create_storage_tables"));
    let migration = files
        .iter()
        .find(|f| f.path == "my-app/db/migration/m20260101000000_create_storage_tables.rs")
        .expect("storage migration file");
    assert!(migration.content.contains("storage_blobs"));
    let dev = files
        .iter()
        .find(|f| f.path == "my-app/config/development.yml")
        .unwrap();
    assert!(dev.content.contains("storage:"));
    assert!(dev.content.contains("service: local"));
}

#[test]
fn test_new_jobs_db_includes_doido_jobs_migration() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite", "--jobs=db"])
        .unwrap();
    let mod_rs = files
        .iter()
        .find(|f| f.path == "my-app/db/migration/mod.rs")
        .unwrap();
    assert!(mod_rs
        .content
        .contains("m20260101000001_create_doido_jobs_table"));
    let migration = files
        .iter()
        .find(|f| f.path == "my-app/db/migration/m20260101000001_create_doido_jobs_table.rs")
        .expect("jobs migration file");
    assert!(migration.content.contains("doido_jobs"));
    assert!(migration.content.contains("run_at BIGINT NOT NULL"));
    assert!(migration.content.contains("locked_at BIGINT"));
    assert!(migration.content.contains("idx_doido_jobs_reserve"));
}

#[test]
fn test_new_jobs_memory_omits_doido_jobs_migration() {
    let files = ProjectGenerator
        .generate(&["my-app", "--database=sqlite", "--jobs=memory"])
        .unwrap();
    let mod_rs = files
        .iter()
        .find(|f| f.path == "my-app/db/migration/mod.rs")
        .unwrap();
    assert!(!mod_rs.content.contains("create_doido_jobs_table"));
    assert!(
        !files
            .iter()
            .any(|f| f.path.contains("create_doido_jobs_table")),
        "jobs migration must not be emitted for memory backend"
    );
}
