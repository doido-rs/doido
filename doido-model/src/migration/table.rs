//! Table operations: `create_table`, `drop_table`, `rename_table`, plus the
//! column-definition builder used inside [`Table::create`].

use sea_orm::sea_query::{
    Alias, ColumnDef, Expr, Table as SqTable, TableCreateStatement, TableDropStatement,
    TableRenameStatement,
};
use sea_orm::{ConnectionTrait, DbErr};

/// Collects column definitions inside [`Table::create`].
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

/// Table-level migration operations.
pub struct Table;

impl Table {
    /// `create_table :name do |t| ... end` — creates a table with an implicit
    /// auto-incrementing `id` primary key plus the columns added in `f`.
    pub async fn create<C: ConnectionTrait>(
        db: &C,
        name: &str,
        f: impl FnOnce(&mut TableBuilder),
    ) -> Result<(), DbErr> {
        db.execute(&create_table_statement(name, f)).await.map(|_| ())
    }

    /// `drop_table :name` — drops the table if it exists.
    pub async fn drop<C: ConnectionTrait>(db: &C, name: &str) -> Result<(), DbErr> {
        db.execute(&drop_table_statement(name)).await.map(|_| ())
    }

    /// `rename_table :from, :to`.
    pub async fn rename<C: ConnectionTrait>(db: &C, from: &str, to: &str) -> Result<(), DbErr> {
        db.execute(&rename_table_statement(from, to)).await.map(|_| ())
    }
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
}
