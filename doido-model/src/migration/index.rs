//! Index operations: `add_index`, `remove_index`.

use crate::sea_orm::sea_query::{
    Alias, Index as SqIndex, IndexCreateStatement, IndexDropStatement,
};
use crate::sea_orm::{ConnectionTrait, DatabaseBackend, DbErr};
use crate::sea_orm_migration::SchemaManager;

use super::opts::{self, CmdOpts};

/// Default index name for a `(table, columns)` pair: `idx_<table>_<col>_<col>`.
fn index_name(table: &str, columns: &[&str]) -> String {
    format!("idx_{table}_{}", columns.join("_"))
}

fn add_index_statement(table: &str, columns: &[&str]) -> IndexCreateStatement {
    let mut stmt = SqIndex::create();
    stmt.name(index_name(table, columns))
        .table(Alias::new(table));
    for col in columns {
        stmt.col(Alias::new(*col));
    }
    stmt
}

fn remove_index_statement(table: &str, columns: &[&str]) -> IndexDropStatement {
    let mut stmt = SqIndex::drop();
    stmt.name(index_name(table, columns))
        .table(Alias::new(table));
    stmt
}

/// `add_index :table, [columns]` — index named `idx_<table>_<cols>`.
pub async fn add_index(
    manager: &SchemaManager<'_>,
    table: &str,
    columns: &[&str],
) -> Result<(), DbErr> {
    add_index_with_opts(manager, table, columns, CmdOpts::default()).await
}

/// Like [`add_index`], with explicit [`CmdOpts`] (e.g. [`CmdOpts::if_not_exists`]).
pub async fn add_index_with_opts(
    manager: &SchemaManager<'_>,
    table: &str,
    columns: &[&str],
    opts: CmdOpts,
) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    let mut stmt = add_index_statement(table, columns);
    if opts.if_not_exists && backend != DatabaseBackend::MySql {
        stmt.if_not_exists();
    }
    match manager.get_connection().execute(&stmt).await {
        Ok(_) => Ok(()),
        Err(e) if opts.if_not_exists && opts::is_duplicate_index_err(backend, &e) => Ok(()),
        Err(e) => Err(e),
    }
}

/// `remove_index :table, [columns]` — drops `idx_<table>_<cols>`.
pub async fn remove_index(
    manager: &SchemaManager<'_>,
    table: &str,
    columns: &[&str],
) -> Result<(), DbErr> {
    remove_index_with_opts(manager, table, columns, CmdOpts::default()).await
}

/// Like [`remove_index`], with explicit [`CmdOpts`] (e.g. [`CmdOpts::if_exists`]).
pub async fn remove_index_with_opts(
    manager: &SchemaManager<'_>,
    table: &str,
    columns: &[&str],
    opts: CmdOpts,
) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    let mut stmt = remove_index_statement(table, columns);
    if opts.if_exists && backend != DatabaseBackend::MySql {
        stmt.if_exists();
    }
    match manager.get_connection().execute(&stmt).await {
        Ok(_) => Ok(()),
        Err(e) if opts.if_exists && opts::is_missing_index_err(backend, &e) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    fn index_name_is_derived_from_table_and_columns() {
        assert_eq!(index_name("users", &["email"]), "idx_users_email");
        assert_eq!(index_name("users", &["a", "b"]), "idx_users_a_b");
    }

    #[test]
    fn add_index_uses_derived_name() {
        let sql = add_index_statement("users", &["email"]).to_string(PostgresQueryBuilder);
        assert!(sql.contains("\"idx_users_email\""));
        assert!(sql.contains("\"users\""));
    }
}
