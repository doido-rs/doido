use crate::generator::{GeneratedFile, Generator};
use crate::generators::field::Field;
use crate::generators::{to_snake, to_table_name};
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
        // Pluralize via the inflector, honouring custom `config/inflection.yaml`
        // rules (e.g. `person` → `people`, uncountables, irregulars).
        let table_name = to_table_name(name);

        // Remaining args are `name:type[:modifier...]` column specs.
        let fields = Field::parse_all(&args[1..])?;

        // Model file — one struct field per declared column.
        let model = MODEL_TEMPLATE
            .replace("{table_name}", &table_name)
            .replace("{fields}", &model_fields(&fields));

        // Migration file. The SeaORM `DeriveMigrationName` derives the name from
        // the module path, so the file/module name doubles as the migration id.
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_create_{table_name}_table");
        let migration = MIGRATION_TEMPLATE
            .replace("{migration_imports}", &migration_imports(&fields))
            .replace("{up_body}", &migration_up_body(&table_name, &fields))
            .replace("{table_name}", &table_name);

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

/// Renders the SeaORM model struct fields (one per line, 4-space indented). The
/// trailing newline keeps the closing `}` of the struct on its own line.
fn model_fields(fields: &[Field]) -> String {
    fields
        .iter()
        .map(|f| format!("    {}\n", f.model_field()))
        .collect()
}

/// The migration crate import line — pulls in `add_index` only when needed so
/// generated code carries no unused imports.
fn migration_imports(fields: &[Field]) -> String {
    if fields.iter().any(Field::wants_index) {
        "use doido_model::migration::{add_index, create_table, drop_table};".to_string()
    } else {
        "use doido_model::migration::{create_table, drop_table};".to_string()
    }
}

/// Builds the body of `up()` — a `create_table` call carrying the declared
/// columns, followed by any `add_index` calls for `:index` fields.
fn migration_up_body(table_name: &str, fields: &[Field]) -> String {
    // No columns: keep the original hint and an unused-arg-safe closure.
    if fields.is_empty() {
        return format!(
            "        // `create_table` adds an auto-incrementing `id` primary key for you.\n\
             \x20       // Add columns with the builder, e.g. `t.string(\"name\").not_null();`.\n\
             \x20       create_table(manager, \"{table_name}\", |_t| {{}}).await\n"
        );
    }

    let columns: String = fields
        .iter()
        .map(|f| format!("            {}\n", f.migration_line()))
        .collect();

    let indexes: Vec<&Field> = fields.iter().filter(|f| f.wants_index()).collect();

    let mut body = String::new();
    body.push_str(
        "        // `create_table` adds an auto-incrementing `id` primary key for you.\n",
    );
    body.push_str(&format!(
        "        create_table(manager, \"{table_name}\", |t| {{\n{columns}        }})\n"
    ));

    if indexes.is_empty() {
        // Return the `create_table` result directly.
        body.push_str("        .await\n");
    } else {
        body.push_str("        .await?;\n");
        for f in indexes {
            body.push_str(&format!(
                "        add_index(manager, \"{table_name}\", &[\"{}\"]).await?;\n",
                f.column_name()
            ));
        }
        body.push_str("        Ok(())\n");
    }

    body
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
