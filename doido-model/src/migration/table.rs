//! Table operations as free functions — [`create_table`], [`drop_table`],
//! [`rename_table`], [`alter_table`] — plus the column-definition builders they
//! use ([`TableBuilder`] for create, [`AlterTableBuilder`] for alter).

use sea_orm::sea_query::{
    Alias, ColumnDef, Expr, Table as SqTable, TableAlterStatement, TableCreateStatement,
    TableDropStatement, TableRenameStatement,
};
use sea_orm::{ConnectionTrait, DbErr};
use sea_orm_migration::SchemaManager;

/// Collects column definitions inside [`create_table`].
///
/// A big-integer, auto-incrementing `id` primary key is added automatically,
/// matching Rails' implicit primary key.
pub struct TableBuilder {
    columns: Vec<ColumnDef>,
}

impl TableBuilder {
    fn new() -> Self {
        let mut id = ColumnDef::new(Alias::new("id"));
        id.big_integer().not_null().auto_increment().primary_key();
        TableBuilder { columns: vec![id] }
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
    Add(ColumnDef),
    Drop(String),
    Rename(String, String),
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
        self.ops.push(AlterOp::Add(col));
        match self.ops.last_mut().expect("just pushed an add op") {
            AlterOp::Add(col) => col,
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
}

fn create_table_statement(name: &str, f: impl FnOnce(&mut TableBuilder)) -> TableCreateStatement {
    let mut builder = TableBuilder::new();
    f(&mut builder);
    let mut stmt = SqTable::create();
    stmt.table(Alias::new(name)).if_not_exists();
    for col in builder.columns {
        stmt.col(col);
    }
    stmt
}

fn drop_table_statement(name: &str) -> TableDropStatement {
    let mut stmt = SqTable::drop();
    stmt.table(Alias::new(name)).if_exists();
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
                    stmt.add_column(col);
                }
                AlterOp::Drop(name) => {
                    stmt.drop_column(Alias::new(name));
                }
                AlterOp::Rename(from, to) => {
                    stmt.rename_column(Alias::new(from), Alias::new(to));
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
    use sea_orm::sea_query::PostgresQueryBuilder;

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
