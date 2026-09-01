//! Shared helpers for emitting SeaORM migration files that call the
//! `doido_model::migration` builders (`create_table`, `alter_table`,
//! `drop_table`, `add_index`, …) instead of raw sea-orm boilerplate.
//!
//! Used by both the model generator (which creates a table for a new model) and
//! the migration generator.

use crate::generators::field::Field;

/// Directory holding the app's migration module sources.
pub const MIGRATION_DIR: &str = "db/migration";

/// Fallback migration `mod.rs` used when the app doesn't have one on disk yet;
/// kept in sync with the generated-app template so injection markers line up.
pub const MIGRATION_MOD_BASE: &str = include_str!("../../templates/new/db/migration/mod.rs");

/// Backward-compatible alias for callers still using the old name.
pub const MIGRATION_SRC_DIR: &str = MIGRATION_DIR;

/// Backward-compatible alias for callers still using the old name.
pub const MIGRATION_LIB_BASE: &str = MIGRATION_MOD_BASE;

/// Renders a full migration file from the imports line and the `up`/`down`
/// bodies (each already indented as an `async fn` body ending in a newline).
pub fn render_migration_file(
    migration_name: &str,
    imports: &str,
    up_body: &str,
    down_body: &str,
) -> String {
    crate::templates::get("migration/migration.rs.template")
        .replace("{migration_name}", migration_name)
        .replace("{migration_imports}", imports)
        .replace("{up_body}", up_body)
        .replace("{down_body}", down_body)
}

/// The `doido_model::migration` import line for a `create_table` migration —
/// pulls in `add_index` only when a field requested an index, so generated code
/// carries no unused imports.
pub fn create_table_imports(fields: &[Field]) -> String {
    if fields.iter().any(Field::wants_index) {
        "use doido::model::migration::{add_index, create_table, drop_table};".to_string()
    } else {
        "use doido::model::migration::{create_table, drop_table};".to_string()
    }
}

/// Builds the `up()` body for a `create_table` migration — a `create_table`
/// call carrying the declared columns, followed by any `add_index` calls for
/// `:index` fields.
pub fn create_table_up(table_name: &str, fields: &[Field]) -> String {
    // No columns: keep the hint and an unused-arg-safe closure.
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

/// The `down()` body that drops a table: `drop_table(manager, "<table>").await`.
pub fn drop_table_down(table_name: &str) -> String {
    format!("        drop_table(manager, \"{table_name}\").await\n")
}

/// Inserts a `mod <module>;` declaration and a `Box::new(<module>::Migration)`
/// registration into `db/migration/mod.rs`, just above the generator markers.
/// Indentation of the list entry mirrors the marker line.
pub fn register_migration(mod_rs: &str, module: &str) -> String {
    let mut lines: Vec<String> = mod_rs.lines().map(String::from).collect();

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
