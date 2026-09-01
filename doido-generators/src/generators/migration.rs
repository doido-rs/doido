use crate::generator::{GeneratedFile, Generator};
use crate::generators::field::Field;
use crate::generators::migration_support::{
    create_table_imports, create_table_up, drop_table_down, register_migration,
    render_migration_file, MIGRATION_LIB_BASE, MIGRATION_SRC_DIR,
};
use crate::generators::to_snake;
use chrono::Utc;
use doido_core::Result;

pub struct MigrationGenerator;

impl Generator for MigrationGenerator {
    fn name(&self) -> &str {
        "migration"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("migration generator requires a name argument")
        })?;
        let snake = to_snake(name);
        // Remaining args are `name:type[:modifier...]` column specs.
        let fields = Field::parse_all(&args[1..])?;

        // Infer the operation from the name (Rails-style) and render the file
        // using the `doido::model::migration` builders.
        let (imports, up_body, down_body) = MigrationOp::parse(&snake).render(&fields);
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_{snake}");
        let migration = render_migration_file(&migration_module, &imports, &up_body, &down_body);

        // The module/file name is the migration id registered in lib.rs.

        // Register the migration in db/migration/mod.rs, preserving any
        // migrations already registered there.
        let mod_path = format!("{MIGRATION_SRC_DIR}/mod.rs");
        let existing =
            std::fs::read_to_string(&mod_path).unwrap_or_else(|_| MIGRATION_LIB_BASE.to_string());
        let mod_rs = register_migration(&existing, &migration_module);

        let test = crate::templates::get("migration/migration_test.rs.template")
            .replace("{snake}", &snake);

        Ok(vec![
            GeneratedFile {
                path: format!("{MIGRATION_SRC_DIR}/{migration_module}.rs"),
                content: migration,
            },
            GeneratedFile {
                path: mod_path,
                content: mod_rs,
            },
            GeneratedFile {
                path: format!("tests/{snake}_migration_test.rs"),
                content: test,
            },
        ])
    }
}

/// The database operation inferred from a Rails-style migration name.
enum MigrationOp {
    /// `create_<table>` — create the table from the field specs.
    CreateTable { table: String },
    /// `drop_<table>` — drop the table (down re-creates it from the fields).
    DropTable { table: String },
    /// `add_<cols>_to_<table>` — add the field columns to an existing table.
    AddColumns { table: String },
    /// `remove_<cols>_from_<table>` — drop the field columns from a table.
    RemoveColumns { table: String },
    /// Unrecognised name — a `create_table` skeleton for the user to fill in.
    Generic { name: String },
}

impl MigrationOp {
    /// Infer the operation from a snake_cased migration name.
    fn parse(snake: &str) -> Self {
        if let Some(table) = snake.strip_prefix("create_") {
            return Self::CreateTable {
                table: table.to_string(),
            };
        }
        if let Some(rest) = snake.strip_prefix("add_") {
            if let Some((_, table)) = rest.rsplit_once("_to_") {
                return Self::AddColumns {
                    table: table.to_string(),
                };
            }
        }
        for prefix in ["remove_", "delete_", "drop_"] {
            if let Some(rest) = snake.strip_prefix(prefix) {
                if let Some((_, table)) = rest.rsplit_once("_from_") {
                    return Self::RemoveColumns {
                        table: table.to_string(),
                    };
                }
            }
        }
        if let Some(table) = snake.strip_prefix("drop_") {
            return Self::DropTable {
                table: table.to_string(),
            };
        }
        Self::Generic {
            name: snake.to_string(),
        }
    }

    /// Render `(imports, up_body, down_body)` for this operation.
    fn render(&self, fields: &[Field]) -> (String, String, String) {
        match self {
            Self::CreateTable { table } | Self::Generic { name: table } => (
                create_table_imports(fields),
                create_table_up(table, fields),
                drop_table_down(table),
            ),
            Self::DropTable { table } => {
                let up = format!("        drop_table(manager, \"{table}\").await\n");
                if fields.is_empty() {
                    let down = format!(
                        "        // TODO: re-create the `{table}` table to make this reversible.\n\
                         \x20       let _ = manager;\n        Ok(())\n"
                    );
                    (
                        "use doido::model::migration::drop_table;".to_string(),
                        up,
                        down,
                    )
                } else {
                    // `down` re-creates the table, so it needs the create imports.
                    (
                        create_table_imports(fields),
                        up,
                        create_table_up(table, fields),
                    )
                }
            }
            Self::AddColumns { table } => (
                "use doido::model::migration::alter_table;".to_string(),
                alter_body(table, &add_lines(fields)),
                alter_body(table, &drop_lines(fields)),
            ),
            Self::RemoveColumns { table } => (
                "use doido::model::migration::alter_table;".to_string(),
                alter_body(table, &drop_lines(fields)),
                alter_body(table, &add_lines(fields)),
            ),
        }
    }
}

fn add_lines(fields: &[Field]) -> Vec<String> {
    fields.iter().map(Field::alter_add_line).collect()
}

fn drop_lines(fields: &[Field]) -> Vec<String> {
    fields.iter().map(Field::alter_drop_line).collect()
}

/// Renders an `alter_table(manager, "<table>", |t| { … }).await` body from the
/// given builder statements. With no statements it leaves a TODO skeleton.
fn alter_body(table: &str, lines: &[String]) -> String {
    if lines.is_empty() {
        return format!(
            "        // TODO: describe the column changes, e.g.\n\
             \x20       // `t.add_column(\"name\", |c| {{ c.string(); }});`.\n\
             \x20       alter_table(manager, \"{table}\", |_t| {{}}).await\n"
        );
    }
    let body: String = lines.iter().map(|l| format!("            {l}\n")).collect();
    format!("        alter_table(manager, \"{table}\", |t| {{\n{body}        }})\n        .await\n")
}
