use crate::generator::{GeneratedFile, Generator};
use crate::generators::field::Field;
use crate::generators::migration_support::{
    create_table_imports, create_table_up, register_migration, MIGRATION_LIB_BASE,
    MIGRATION_SRC_DIR,
};
use crate::generators::{to_pascal, to_snake, to_table_name};
use chrono::Utc;
use doido_core::Result;

/// Fallback `app/models/mod.rs` used when the app doesn't have one on disk yet.
const MODELS_MOD_BASE: &str = include_str!("../../templates/new/app/models/mod.rs");
/// Path to the application models module registry.
const MODELS_MOD_PATH: &str = "app/models/mod.rs";

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
        // Pluralize via the inflector, honouring custom `config/inflection.yaml`
        // rules (e.g. `person` → `people`, uncountables, irregulars).
        let table_name = to_table_name(name);

        // Remaining args are `name:type[:modifier...]` column specs.
        let fields = Field::parse_all(&args[1..])?;

        // Model file — one struct field per declared column.
        let model = crate::templates::get("models/model.rs.template")
            .replace("{table_name}", &table_name)
            .replace("{fields}", &model_fields(&fields));

        // Migration file. The module/file name is the migration id (`MigrationName::name`).
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_create_{table_name}_table");
        let migration = crate::templates::get("models/migration.rs.template")
            .replace("{migration_name}", &migration_module)
            .replace("{migration_imports}", &create_table_imports(&fields))
            .replace("{up_body}", &create_table_up(&table_name, &fields))
            .replace("{table_name}", &table_name);

        // Register the migration in db/migration/src/lib.rs, preserving any
        // migrations already registered there.
        let lib_path = format!("{MIGRATION_SRC_DIR}/lib.rs");
        let existing =
            std::fs::read_to_string(&lib_path).unwrap_or_else(|_| MIGRATION_LIB_BASE.to_string());
        let lib = register_migration(&existing, &migration_module);

        // Register the model's module in app/models/mod.rs, preserving existing
        // registrations.
        let models_mod_existing = std::fs::read_to_string(MODELS_MOD_PATH)
            .unwrap_or_else(|_| MODELS_MOD_BASE.to_string());
        let models_mod = register_model_module(&models_mod_existing, &snake);

        // Model test stub (a standalone integration test target — a TODO
        // placeholder needs no imports, so it compiles in the binary app crate).
        let model_test = crate::templates::get("models/model_test.rs.template")
            .replace("{Model}", &to_pascal(name))
            .replace("{singular}", &snake);

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
            GeneratedFile {
                path: MODELS_MOD_PATH.to_string(),
                content: models_mod,
            },
            GeneratedFile {
                path: format!("tests/{snake}_model_test.rs"),
                content: model_test,
            },
        ])
    }
}

/// Renders the SeaORM model struct fields (one per line, 4-space indented). The
/// trailing newline keeps the closing `}` of the struct on its own line.
fn model_fields(fields: &[Field]) -> String {
    fields
        .iter()
        .map(|f| format!("    {}\n", f.model_field()))
        .collect()
}

/// Inserts `pub mod <module>;` into `app/models/mod.rs` just above the
/// `@generated-models` marker. Idempotent: if the module is already registered,
/// the file is returned unchanged.
fn register_model_module(models_mod: &str, module: &str) -> String {
    let decl = format!("pub mod {module};");
    if models_mod.lines().any(|l| l.trim() == decl) {
        return models_mod.to_string();
    }

    let mut lines: Vec<String> = models_mod.lines().map(String::from).collect();
    if let Some(i) = lines.iter().position(|l| l.contains("@generated-models")) {
        lines.insert(i, decl);
    } else {
        lines.push(decl);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
