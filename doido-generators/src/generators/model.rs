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
/// Fallback `_entities/mod.rs` used when the app doesn't have one on disk yet.
const ENTITIES_MOD_BASE: &str = include_str!("../../templates/new/app/models/_entities/mod.rs");
/// Path to the application models module registry.
const MODELS_MOD_PATH: &str = "app/models/mod.rs";
const ENTITIES_MOD_PATH: &str = "app/models/_entities/mod.rs";

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
        let model_name = to_pascal(name);
        // Pluralize via the inflector, honouring custom `config/inflection.yaml`
        // rules (e.g. `person` → `people`, uncountables, irregulars).
        let table_name = to_table_name(name);

        // Remaining args are `name:type[:modifier...]` column specs.
        let fields = Field::parse_all(&args[1..])?;

        let entity = crate::templates::get("models/entity.rs.template")
            .replace("{table_name}", &table_name)
            .replace("{fields}", &model_fields(&fields));

        let extension = crate::templates::get("models/model.rs.template")
            .replace("{Model}", &model_name)
            .replace("{table_name}", &table_name);

        // Migration file. The module/file name is the migration id (`MigrationName::name`).
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_create_{table_name}_table");
        let migration = crate::templates::get("models/migration.rs.template")
            .replace("{migration_name}", &migration_module)
            .replace("{migration_imports}", &create_table_imports(&fields))
            .replace("{up_body}", &create_table_up(&table_name, &fields))
            .replace("{table_name}", &table_name);

        // Register the migration in db/migration/mod.rs, preserving any
        // migrations already registered there.
        let mod_path = format!("{MIGRATION_SRC_DIR}/mod.rs");
        let existing =
            std::fs::read_to_string(&mod_path).unwrap_or_else(|_| MIGRATION_LIB_BASE.to_string());
        let mod_rs = register_migration(&existing, &migration_module);

        // Register the model's module in app/models/mod.rs, preserving existing
        // registrations.
        let models_mod_existing = std::fs::read_to_string(MODELS_MOD_PATH)
            .unwrap_or_else(|_| MODELS_MOD_BASE.to_string());
        let models_mod = register_model_module(&models_mod_existing, &snake);

        let entities_mod_existing = std::fs::read_to_string(ENTITIES_MOD_PATH)
            .unwrap_or_else(|_| ENTITIES_MOD_BASE.to_string());
        let entities_mod = register_entity_module(&entities_mod_existing, &table_name);

        // Model test stub (a standalone integration test target — a TODO
        // placeholder needs no imports, so it compiles in the binary app crate).
        let model_test = crate::templates::get("models/model_test.rs.template")
            .replace("{Model}", &model_name)
            .replace("{singular}", &snake);

        Ok(vec![
            GeneratedFile {
                path: format!("app/models/_entities/{table_name}.rs"),
                content: entity,
            },
            GeneratedFile {
                path: format!("app/models/{snake}.rs"),
                content: extension,
            },
            GeneratedFile {
                path: ENTITIES_MOD_PATH.to_string(),
                content: entities_mod,
            },
            GeneratedFile {
                path: format!("{MIGRATION_SRC_DIR}/{migration_module}.rs"),
                content: migration,
            },
            GeneratedFile {
                path: mod_path,
                content: mod_rs,
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
    doido_model::entities::register_model_module(models_mod, module)
}

/// Inserts `pub mod <module>;` into `_entities/mod.rs` just above the marker.
fn register_entity_module(entities_mod: &str, module: &str) -> String {
    doido_model::entities::register_entity_module(entities_mod, module)
}
