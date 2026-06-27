use crate::generator::{GeneratedFile, Generator};
use crate::generators::to_snake;
use chrono::Utc;
use doido_core::Result;

/// Model entity, written to `app/models/<name>.rs`.
const MODEL_TEMPLATE: &str = include_str!("../../templates/models/model.rs.template");
/// SeaORM migration, written to `db/migration/src/m<timestamp>_create_<table>_table.rs`.
const MIGRATION_TEMPLATE: &str = include_str!("../../templates/models/migration.rs.template");
/// Fallback migration `lib.rs` used when the app doesn't have one on disk yet;
/// kept in sync with the generated-app template so injection markers line up.
const MIGRATION_LIB_BASE: &str = include_str!("../../templates/new/db/migration/src/lib.rs");

/// Directory holding the SeaORM migration crate's sources.
const MIGRATION_SRC_DIR: &str = "db/migration/src";

pub struct ModelGenerator;

impl Generator for ModelGenerator {
    fn name(&self) -> &str {
        "model"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("model generator requires a name argument")
        })?;
        let snake = to_snake(name);
        // Naive pluralization, matching the rest of the generators.
        let table_name = format!("{snake}s");

        // Model file.
        let model = MODEL_TEMPLATE.replace("{table_name}", &table_name);

        // Migration file. The SeaORM `DeriveMigrationName` derives the name from
        // the module path, so the file/module name doubles as the migration id.
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_create_{table_name}_table");
        let migration = MIGRATION_TEMPLATE.replace("{table_name}", &table_name);

        // Register the migration in db/migration/src/lib.rs, preserving any
        // migrations already registered there.
        let lib_path = format!("{MIGRATION_SRC_DIR}/lib.rs");
        let existing =
            std::fs::read_to_string(&lib_path).unwrap_or_else(|_| MIGRATION_LIB_BASE.to_string());
        let lib = register_migration(&existing, &migration_module);

        Ok(vec![
            GeneratedFile {
                path: format!("app/models/{snake}.rs"),
                content: model,
            },
            GeneratedFile {
                path: format!("{MIGRATION_SRC_DIR}/{migration_module}.rs"),
                content: migration,
            },
            GeneratedFile {
                path: lib_path,
                content: lib,
            },
        ])
    }
}

/// Inserts a `mod <module>;` declaration and a `Box::new(<module>::Migration)`
/// registration into the migration crate's `lib.rs`, just above the generator
/// markers. Indentation of the list entry mirrors the marker line.
fn register_migration(lib: &str, module: &str) -> String {
    let mut lines: Vec<String> = lib.lines().map(String::from).collect();

    if let Some(i) = lines
        .iter()
        .position(|l| l.contains("@generated-migrations-mod"))
    {
        lines.insert(i, format!("mod {module};"));
    }

    if let Some(i) = lines
        .iter()
        .position(|l| l.contains("@generated-migrations-list"))
    {
        let indent: String = lines[i].chars().take_while(|c| c.is_whitespace()).collect();
        lines.insert(i, format!("{indent}Box::new({module}::Migration),"));
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}
