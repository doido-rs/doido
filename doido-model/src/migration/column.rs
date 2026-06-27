//! Column operations: `add_column`, `remove_column`, `rename_column`.

use sea_orm::sea_query::{Alias, ColumnDef, Table as SqTable, TableAlterStatement};
use sea_orm::{ConnectionTrait, DbErr};

fn add_column_statement(
    table: &str,
    name: &str,
    f: impl FnOnce(&mut ColumnDef),
) -> TableAlterStatement {
    let mut col = ColumnDef::new(Alias::new(name));
    f(&mut col);
    let mut stmt = SqTable::alter();
    stmt.table(Alias::new(table)).add_column(col);
    stmt
}

fn remove_column_statement(table: &str, name: &str) -> TableAlterStatement {
    let mut stmt = SqTable::alter();
    stmt.table(Alias::new(table)).drop_column(Alias::new(name));
    stmt
}

fn rename_column_statement(table: &str, from: &str, to: &str) -> TableAlterStatement {
    let mut stmt = SqTable::alter();
    stmt.table(Alias::new(table))
        .rename_column(Alias::new(from), Alias::new(to));
    stmt
}

/// Column-level migration operations.
pub struct Column;

impl Column {
    /// `add_column :table, :name, :type` — `f` configures the column type/modifiers.
    pub async fn add<C: ConnectionTrait>(
        db: &C,
        table: &str,
        name: &str,
        f: impl FnOnce(&mut ColumnDef),
    ) -> Result<(), DbErr> {
        db.execute(&add_column_statement(table, name, f))
            .await
            .map(|_| ())
    }

    /// `remove_column :table, :name`.
    pub async fn remove<C: ConnectionTrait>(db: &C, table: &str, name: &str) -> Result<(), DbErr> {
        db.execute(&remove_column_statement(table, name))
            .await
            .map(|_| ())
    }

    /// `rename_column :table, :from, :to`.
    pub async fn rename<C: ConnectionTrait>(
        db: &C,
        table: &str,
        from: &str,
        to: &str,
    ) -> Result<(), DbErr> {
        db.execute(&rename_column_statement(table, from, to))
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    fn add_column_alters_table() {
        let sql = add_column_statement("users", "age", |c| {
            c.integer();
        })
        .to_string(PostgresQueryBuilder);
        assert!(sql.contains("ALTER TABLE \"users\""));
        assert!(sql.contains("\"age\""));
    }

    #[test]
    fn rename_column_renders_both_names() {
        let sql = rename_column_statement("users", "sku", "code").to_string(PostgresQueryBuilder);
        assert!(sql.contains("\"sku\""));
        assert!(sql.contains("\"code\""));
    }
}
