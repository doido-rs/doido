use doido_generators::commands::generate::{registry_for_project_at_with, run_generate};
use doido_generators::generators::field::Field;
use doido_generators::generators::migration_support::{
    create_table_up, register_migration, MIGRATION_MOD_BASE,
};
use doido_generators::templates;
use doido_generators::{GeneratedFile, Generator};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn every_builtin_template_resolves() {
    for (rel, default) in templates::builtin_templates() {
        let content = templates::get_with_root(Path::new("/nonexistent"), rel);
        assert_eq!(content, *default);
        assert!(!content.is_empty(), "empty template for {rel}");
    }
}

#[test]
fn create_table_up_with_index_fields_ends_with_ok() {
    let fields = Field::parse_all(&["email:string:index"]).unwrap();
    let up = create_table_up("users", &fields);
    assert!(up.contains("add_index(manager, \"users\""));
    assert!(up.contains("Ok(())"));
}

#[test]
fn register_migration_without_markers_is_noop() {
    let base = "pub struct Migrator;\n";
    let updated = register_migration(base, "m20260101_create_widgets_table");
    assert_eq!(updated, base);
}

#[test]
fn register_migration_on_template_base_preserves_markers() {
    let updated = register_migration(MIGRATION_MOD_BASE, "m20260101_create_widgets_table");
    assert!(updated.contains("@generated-migrations-mod"));
    assert!(updated.contains("@generated-migrations-list"));
}

#[test]
fn run_generate_model_in_temp_project() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("db/migration")).unwrap();
    fs::write(dir.path().join("db/migration/mod.rs"), MIGRATION_MOD_BASE).unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();

    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    run_generate("model", &["Widget", "name:string"]);
    assert!(dir.path().join("app/models/widget.rs").exists());
    assert!(dir.path().join("app/models/_entities/widgets.rs").exists());
    std::env::set_current_dir(original).unwrap();
}

struct EchoGenerator;

impl Generator for EchoGenerator {
    fn name(&self) -> &str {
        "echo"
    }

    fn generate(&self, args: &[&str]) -> doido_core::Result<Vec<GeneratedFile>> {
        Ok(vec![GeneratedFile {
            path: format!("generated/{}.txt", args.first().unwrap_or(&"x")),
            content: "echo".into(),
        }])
    }
}

#[test]
fn registry_for_project_at_with_merges_custom_generators() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let reg = registry_for_project_at_with(dir.path(), vec![Box::new(EchoGenerator)]);
    assert!(reg.list().contains(&"echo"));
    assert!(reg.list().contains(&"model"));
}

#[test]
fn custom_generator_wins_on_name_collision() {
    struct OverrideModel;

    impl Generator for OverrideModel {
        fn name(&self) -> &str {
            "model"
        }

        fn generate(&self, _args: &[&str]) -> doido_core::Result<Vec<GeneratedFile>> {
            Ok(vec![GeneratedFile {
                path: "custom.txt".into(),
                content: "override".into(),
            }])
        }
    }

    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let reg = registry_for_project_at_with(dir.path(), vec![Box::new(OverrideModel)]);
    let files = reg.run("model", &["X"]).unwrap();
    assert_eq!(files[0].path, "custom.txt");
}
