//! Column operations: `add_column`, `remove_column`, `rename_column`.

use crate::sea_orm::sea_query::{Alias, ColumnDef, Table as SqTable, TableAlterStatement};
use crate::sea_orm::{ConnectionTrait, DbErr};
use crate::sea_orm_migration::SchemaManager;

use super::opts::{self, CmdOpts};

fn add_column_statement(
    table: &str,
    name: &str,
    f: impl FnOnce(&mut ColumnDef),
    opts: CmdOpts,
) -> TableAlterStatement {
    let mut col = ColumnDef::new(Alias::new(name));
    f(&mut col);
    let mut stmt = SqTable::alter();
    stmt.table(Alias::new(table));
    if opts.if_not_exists {
        stmt.add_column_if_not_exists(col);
    } else {
        stmt.add_column(col);
    }
    stmt
}

fn remove_column_statement(table: &str, name: &str, opts: CmdOpts) -> TableAlterStatement {
    let mut stmt = SqTable::alter();
    stmt.table(Alias::new(table));
    if opts.if_exists {
        stmt.drop_column_if_exists(Alias::new(name));
    } else {
        stmt.drop_column(Alias::new(name));
    }
    stmt
}

fn rename_column_statement(table: &str, from: &str, to: &str) -> TableAlterStatement {
    let mut stmt = SqTable::alter();
    stmt.table(Alias::new(table))
        .rename_column(Alias::new(from), Alias::new(to));
    stmt
}

/// `add_column :table, :name, :type` — `f` configures the column type/modifiers.
pub async fn add_column(
    manager: &SchemaManager<'_>,
    table: &str,
    name: &str,
    f: impl FnOnce(&mut ColumnDef),
) -> Result<(), DbErr> {
    add_column_with_opts(manager, table, name, f, CmdOpts::default()).await
}

/// Like [`add_column`], with explicit [`CmdOpts`].
pub async fn add_column_with_opts(
    manager: &SchemaManager<'_>,
    table: &str,
    name: &str,
    f: impl FnOnce(&mut ColumnDef),
    opts: CmdOpts,
) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    match manager
        .get_connection()
        .execute(&add_column_statement(table, name, f, opts))
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if opts.if_not_exists && opts::is_duplicate_column_err(backend, &e) => Ok(()),
        Err(e) => Err(e),
    }
}

/// `remove_column :table, :name`.
pub async fn remove_column(
    manager: &SchemaManager<'_>,
    table: &str,
    name: &str,
) -> Result<(), DbErr> {
    remove_column_with_opts(manager, table, name, CmdOpts::default()).await
}

/// Like [`remove_column`], with explicit [`CmdOpts`].
pub async fn remove_column_with_opts(
    manager: &SchemaManager<'_>,
    table: &str,
    name: &str,
    opts: CmdOpts,
) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    match manager
        .get_connection()
        .execute(&remove_column_statement(table, name, opts))
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if opts.if_exists && opts::is_missing_column_err(backend, &e) => Ok(()),
        Err(e) => Err(e),
    }
}

/// `rename_column :table, :from, :to`.
pub async fn rename_column(
    manager: &SchemaManager<'_>,
    table: &str,
    from: &str,
    to: &str,
) -> Result<(), DbErr> {
    rename_column_with_opts(manager, table, from, to, CmdOpts::default()).await
}

/// Like [`rename_column`], with explicit [`CmdOpts`] (currently ignored).
pub async fn rename_column_with_opts(
    manager: &SchemaManager<'_>,
    table: &str,
    from: &str,
    to: &str,
    _opts: CmdOpts,
) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&rename_column_statement(table, from, to))
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    fn add_column_alters_table() {
        let sql = add_column_statement(
            "users",
            "age",
            |c| {
                c.integer();
            },
            CmdOpts::new(),
        )
        .to_string(PostgresQueryBuilder);
        assert!(sql.contains("ALTER TABLE \"users\""));
        assert!(sql.contains("\"age\""));
    }

    #[test]
    fn add_column_if_not_exists() {
        let sql = add_column_statement(
            "users",
            "age",
            |c| {
                c.integer();
            },
            CmdOpts::if_not_exists(),
        )
        .to_string(PostgresQueryBuilder);
        assert!(sql.to_uppercase().contains("IF NOT EXISTS"));
    }

    #[test]
    fn rename_column_renders_both_names() {
        let sql = rename_column_statement("users", "sku", "code").to_string(PostgresQueryBuilder);
        assert!(sql.contains("\"sku\""));
        assert!(sql.contains("\"code\""));
    }
}
