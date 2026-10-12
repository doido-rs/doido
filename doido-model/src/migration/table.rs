//! Table operations as free functions — [`create_table`], [`drop_table`],
//! [`rename_table`], [`alter_table`] — plus the column-definition builders they
//! use ([`TableBuilder`] for create, [`AlterTableBuilder`] for alter).

use super::foreign_key::{add_foreign_key_statement, ForeignKeyOpts};
use crate::sea_orm::sea_query::{
    Alias, ColumnDef, Expr, SimpleExpr, Table as SqTable, TableAlterStatement,
    TableCreateStatement, TableDropStatement, TableRenameStatement,
};
use crate::sea_orm::{ConnectionTrait, DbErr};
use crate::sea_orm_migration::SchemaManager;

/// Options for [`TableBuilder::references_named`].
#[derive(Debug, Clone)]
pub struct ReferenceOpts {
    pub to_column: String,
    pub foreign_key_name: Option<String>,
    pub on_delete: Option<crate::sea_orm::sea_query::ForeignKeyAction>,
    pub on_update: Option<crate::sea_orm::sea_query::ForeignKeyAction>,
    pub nullable: bool,
}

impl Default for ReferenceOpts {
    fn default() -> Self {
        Self {
            to_column: "id".into(),
            foreign_key_name: None,
            on_delete: None,
            on_update: None,
            nullable: false,
        }
    }
}

struct NamedCheck {
    expr: SimpleExpr,
}

struct PendingReference {
    from_column: String,
    to_table: String,
    to_column: String,
    opts: ForeignKeyOpts,
}

/// Collects column definitions inside [`create_table`].
///
/// A big-integer, auto-incrementing `id` primary key is added automatically,
/// matching Rails' implicit primary key. Call [`TableBuilder::without_implicit_id`]
/// and [`TableBuilder::serial_id`] for integer `serial` primary keys (Loco/Fivia style).
pub struct TableBuilder {
    columns: Vec<ColumnDef>,
    checks: Vec<NamedCheck>,
    references: Vec<PendingReference>,
    implicit_id: bool,
}

impl TableBuilder {
    fn new() -> Self {
        let mut builder = TableBuilder {
            columns: Vec::new(),
            checks: Vec::new(),
            references: Vec::new(),
            implicit_id: true,
        };
        builder.push_implicit_id();
        builder
    }

    fn push_implicit_id(&mut self) {
        let mut id = ColumnDef::new(Alias::new("id"));
        id.big_integer().not_null().auto_increment().primary_key();
        self.columns.push(id);
    }

    /// Omit the default bigint `id` column (use before [`Self::serial_id`] or a custom PK).
    pub fn without_implicit_id(&mut self) {
        if self.implicit_id {
            self.implicit_id = false;
            if !self.columns.is_empty() {
                self.columns.remove(0);
            }
        }
    }

    /// Integer `serial` primary key — PostgreSQL-compatible with legacy Fivia/Loco tables.
    pub fn serial_id(&mut self) -> &mut ColumnDef {
        self.without_implicit_id();
        self.typed("id", |c| {
            c.integer().not_null().auto_increment().primary_key();
        })
    }

    /// Table-level `CHECK` constraint. For a named PostgreSQL constraint, pass
    /// `Expr::cust(r#"CONSTRAINT "name" CHECK (…)"#)` as `expr`.
    pub fn check(&mut self, _name: Option<&str>, expr: SimpleExpr) {
        self.checks.push(NamedCheck { expr });
    }

    /// Adds `<name>_id` and an embedded foreign key for [`create_table`].
    pub fn references_named(
        &mut self,
        name: &str,
        to_table: &str,
        opts: ReferenceOpts,
    ) -> &mut ColumnDef {
        let col_name = format!("{name}_id");
        self.references.push(PendingReference {
            from_column: col_name.clone(),
            to_table: to_table.to_string(),
            to_column: opts.to_column,
            opts: ForeignKeyOpts {
                name: opts.foreign_key_name,
                on_delete: opts.on_delete,
                on_update: opts.on_update,
            },
        });
        self.typed(&col_name, |c| {
            c.integer();
            if !opts.nullable {
                c.not_null();
            }
        })
    }

    /// Pushes a fully built column and returns it for further chaining
    /// (e.g. `.not_null()`, `.default(...)`, `.unique_key()`).
    pub fn column(&mut self, col: ColumnDef) -> &mut ColumnDef {
        self.columns.push(col);
        self.columns.last_mut().expect("just pushed a column")
    }

    fn typed(&mut self, name: &str, apply: impl FnOnce(&mut ColumnDef)) -> &mut ColumnDef {
        let mut col = ColumnDef::new(Alias::new(name));
        apply(&mut col);
        self.column(col)
    }

    pub fn string(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.string();
        })
    }
    pub fn text(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.text();
        })
    }
    pub fn integer(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.integer();
        })
    }
    pub fn small_integer(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.small_integer();
        })
    }
    pub fn big_integer(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.big_integer();
        })
    }
    pub fn float(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.float();
        })
    }
    pub fn double(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.double();
        })
    }
    pub fn decimal(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.decimal();
        })
    }
    pub fn boolean(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.boolean();
        })
    }
    pub fn timestamp(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.timestamp();
        })
    }
    pub fn timestamp_with_time_zone(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.timestamp_with_time_zone();
        })
    }
    pub fn timestamptz(&mut self, name: &str) -> &mut ColumnDef {
        self.timestamp_with_time_zone(name)
    }
    pub fn time(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.time();
        })
    }
    pub fn date(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.date();
        })
    }
    pub fn json(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.json();
        })
    }
    pub fn json_binary(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.json_binary();
        })
    }
    pub fn uuid(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.uuid();
        })
    }
    pub fn binary(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(name, |c| {
            c.binary();
        })
    }

    /// Rails `t.references :user` — adds a non-null `<name>_id` big-integer column.
    pub fn references(&mut self, name: &str) -> &mut ColumnDef {
        self.typed(&format!("{name}_id"), |c| {
            c.big_integer().not_null();
        })
    }

    /// Rails `t.timestamps` — adds non-null `created_at` and `updated_at`,
    /// each defaulting to the current timestamp.
    pub fn timestamps(&mut self) {
        self.typed("created_at", |c| {
            c.timestamp().not_null().default(Expr::current_timestamp());
        });
        self.typed("updated_at", |c| {
            c.timestamp().not_null().default(Expr::current_timestamp());
        });
    }
}

/// One pending change collected by [`AlterTableBuilder`].
enum AlterOp {
    Add(Box<ColumnDef>),
    Drop(String),
    Rename(String, String),
    Modify(Box<ColumnDef>),
}

/// Collects changes inside [`alter_table`]. Each change is applied as its own
/// `ALTER TABLE` statement, so this works uniformly across backends (including
/// SQLite, which permits only one alteration per statement).
pub struct AlterTableBuilder {
    ops: Vec<AlterOp>,
}

impl AlterTableBuilder {
    fn new() -> Self {
        AlterTableBuilder { ops: Vec::new() }
    }

    /// `add_column :name` — `f` configures the column type and modifiers. The
    /// returned `&mut ColumnDef` allows further chaining.
    pub fn add_column(&mut self, name: &str, f: impl FnOnce(&mut ColumnDef)) -> &mut ColumnDef {
        let mut col = ColumnDef::new(Alias::new(name));
        f(&mut col);
        self.ops.push(AlterOp::Add(Box::new(col)));
        match self.ops.last_mut().expect("just pushed an add op") {
            AlterOp::Add(col) => col.as_mut(),
            _ => unreachable!("last op is the add we just pushed"),
        }
    }

    /// `drop_column :name`.
    pub fn drop_column(&mut self, name: &str) {
        self.ops.push(AlterOp::Drop(name.to_string()));
    }

    /// `rename_column :from, :to`.
    pub fn rename_column(&mut self, from: &str, to: &str) {
        self.ops
            .push(AlterOp::Rename(from.to_string(), to.to_string()));
    }

    /// `ALTER COLUMN … SET NOT NULL` (PostgreSQL) / equivalent via `MODIFY` on MySQL.
    pub fn set_not_null(&mut self, col: &str) {
        let mut c = ColumnDef::new(Alias::new(col));
        c.not_null();
        self.ops.push(AlterOp::Modify(Box::new(c)));
    }

    /// `ALTER COLUMN … DROP NOT NULL`.
    pub fn drop_not_null(&mut self, col: &str) {
        let mut c = ColumnDef::new(Alias::new(col));
        c.null();
        self.ops.push(AlterOp::Modify(Box::new(c)));
    }

    /// `ALTER COLUMN … SET DEFAULT …`.
    pub fn set_default(&mut self, col: &str, default: SimpleExpr) {
        let mut c = ColumnDef::new(Alias::new(col));
        c.default(default);
        self.ops.push(AlterOp::Modify(Box::new(c)));
    }

    /// `ALTER COLUMN … DROP DEFAULT` (PostgreSQL-style via SeaQuery `Extra`).
    pub fn drop_default(&mut self, col: &str) {
        let mut c = ColumnDef::new(Alias::new(col));
        c.extra(format!(
            "ALTER COLUMN \"{}\" DROP DEFAULT",
            col.replace('"', "\"\"")
        ));
        self.ops.push(AlterOp::Modify(Box::new(c)));
    }

    /// `ADD CONSTRAINT name CHECK (…)` on PostgreSQL (via SeaQuery `Check` alter).
    pub fn add_check(&mut self, name: &str, expr: SimpleExpr) {
        let mut c = ColumnDef::new(Alias::new(name));
        c.check(expr);
        self.ops.push(AlterOp::Modify(Box::new(c)));
    }
}

fn create_table_statement(name: &str, f: impl FnOnce(&mut TableBuilder)) -> TableCreateStatement {
    let mut builder = TableBuilder::new();
    f(&mut builder);
    let mut stmt = SqTable::create();
    stmt.table(Alias::new(name)).if_not_exists();
    for col in builder.columns {
        stmt.col(col);
    }
    for check in builder.checks {
        stmt.check(check.expr);
    }
    for reference in builder.references {
        let mut fk = add_foreign_key_statement(
            name,
            &reference.from_column,
            &reference.to_table,
            &reference.to_column,
            &reference.opts,
        );
        stmt.foreign_key(&mut fk);
    }
    stmt
}

fn drop_table_statement(name: &str) -> TableDropStatement {
    let mut stmt = SqTable::drop();
    stmt.table(Alias::new(name)).if_exists();
    stmt
}

fn drop_table_cascade_statement(name: &str) -> TableDropStatement {
    let mut stmt = SqTable::drop();
    stmt.table(Alias::new(name)).if_exists().cascade();
    stmt
}

fn rename_table_statement(from: &str, to: &str) -> TableRenameStatement {
    let mut stmt = SqTable::rename();
    stmt.table(Alias::new(from), Alias::new(to));
    stmt
}

fn alter_table_statements(name: &str, ops: Vec<AlterOp>) -> Vec<TableAlterStatement> {
    ops.into_iter()
        .map(|op| {
            let mut stmt = SqTable::alter();
            stmt.table(Alias::new(name));
            match op {
                AlterOp::Add(col) => {
                    stmt.add_column(*col);
                }
                AlterOp::Drop(name) => {
                    stmt.drop_column(Alias::new(name));
                }
                AlterOp::Rename(from, to) => {
                    stmt.rename_column(Alias::new(from), Alias::new(to));
                }
                AlterOp::Modify(col) => {
                    stmt.modify_column(*col);
                }
            }
            stmt
        })
        .collect()
}

/// `create_table :name do |t| ... end` — creates a table with an implicit
/// auto-incrementing `id` primary key plus the columns added in `f`.
pub async fn create_table(
    manager: &SchemaManager<'_>,
    name: &str,
    f: impl FnOnce(&mut TableBuilder),
) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&create_table_statement(name, f))
        .await
        .map(|_| ())
}

/// `drop_table :name` — drops the table if it exists.
pub async fn drop_table(manager: &SchemaManager<'_>, name: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&drop_table_statement(name))
        .await
        .map(|_| ())
}

/// `DROP TABLE IF EXISTS name CASCADE`.
pub async fn drop_table_cascade(manager: &SchemaManager<'_>, name: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&drop_table_cascade_statement(name))
        .await
        .map(|_| ())
}

/// `rename_table :from, :to`.
pub async fn rename_table(manager: &SchemaManager<'_>, from: &str, to: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&rename_table_statement(from, to))
        .await
        .map(|_| ())
}

/// `alter_table :name do |t| ... end` — applies the column changes collected in
/// `f` (add/drop/rename), each as its own `ALTER TABLE` statement.
pub async fn alter_table(
    manager: &SchemaManager<'_>,
    name: &str,
    f: impl FnOnce(&mut AlterTableBuilder),
) -> Result<(), DbErr> {
    let mut builder = AlterTableBuilder::new();
    f(&mut builder);
    let conn = manager.get_connection();
    for stmt in alter_table_statements(name, builder.ops) {
        conn.execute(&stmt).await.map(|_| ())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    fn create_table_adds_implicit_id_and_columns() {
        let sql = create_table_statement("users", |t| {
            t.string("email").not_null();
            t.timestamps();
        })
        .to_string(PostgresQueryBuilder);
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"users\""));
        assert!(sql.contains("\"id\""));
        assert!(sql.contains("\"email\""));
        assert!(sql.contains("\"created_at\""));
        assert!(sql.contains("\"updated_at\""));
    }

    #[test]
    fn serial_id_uses_integer_serial_on_postgres() {
        let sql = create_table_statement("legacy", |t| {
            t.without_implicit_id();
            t.serial_id();
            t.string("name");
        })
        .to_string(PostgresQueryBuilder);
        assert!(
            sql.contains("serial") || sql.contains("\"id\" integer"),
            "expected serial or integer id, got: {sql}"
        );
        assert!(!sql.contains("bigserial"));
    }

    #[test]
    fn extra_column_types_render() {
        let sql = create_table_statement("slots", |t| {
            t.time("starts_at");
            t.small_integer("weekday");
            t.json_binary("meta");
            t.timestamp_with_time_zone("ends_at");
        })
        .to_string(PostgresQueryBuilder);
        assert!(sql.contains("\"starts_at\""));
        assert!(sql.contains("jsonb"));
        assert!(sql.contains("time zone"));
    }

    #[test]
    fn drop_table_cascade_includes_cascade() {
        let sql = drop_table_cascade_statement("old_billing").to_string(PostgresQueryBuilder);
        assert!(sql.contains("DROP TABLE IF EXISTS \"old_billing\""));
        assert!(sql.contains("CASCADE"));
    }

    #[test]
    fn set_not_null_alter_renders() {
        let mut builder = AlterTableBuilder::new();
        builder.set_not_null("email");
        let stmts = alter_table_statements("users", builder.ops);
        let sql = stmts[0].to_string(PostgresQueryBuilder);
        assert!(sql.contains("SET NOT NULL"));
    }

    #[test]
    fn references_adds_id_suffixed_column() {
        let sql = create_table_statement("comments", |t| {
            t.references("post");
        })
        .to_string(PostgresQueryBuilder);
        assert!(sql.contains("\"post_id\""));
    }

    #[test]
    fn drop_table_is_conditional() {
        let sql = drop_table_statement("users").to_string(PostgresQueryBuilder);
        assert!(sql.contains("DROP TABLE IF EXISTS \"users\""));
    }

    #[test]
    fn alter_table_emits_one_statement_per_change() {
        let mut builder = AlterTableBuilder::new();
        builder.add_column("age", |c| {
            c.integer();
        });
        builder.drop_column("legacy");
        builder.rename_column("sku", "code");

        let stmts = alter_table_statements("items", builder.ops);
        assert_eq!(stmts.len(), 3);
        let add = stmts[0].to_string(PostgresQueryBuilder);
        assert!(add.contains("ALTER TABLE \"items\""));
        assert!(add.contains("\"age\""));
        assert!(stmts[1]
            .to_string(PostgresQueryBuilder)
            .contains("\"legacy\""));
        let rename = stmts[2].to_string(PostgresQueryBuilder);
        assert!(rename.contains("\"sku\""));
        assert!(rename.contains("\"code\""));
    }
}
