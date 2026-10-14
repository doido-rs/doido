//! Foreign-key operations: `add_foreign_key`, `remove_foreign_key`.

use crate::sea_orm::sea_query::{
    Alias, ForeignKey as SqForeignKey, ForeignKeyCreateStatement, ForeignKeyDropStatement,
};
use crate::sea_orm::{ConnectionTrait, DbErr};
use crate::sea_orm_migration::SchemaManager;

use super::opts::{self, CmdOpts};

/// Default foreign-key constraint name: `fk_<table>_<column>`.
fn foreign_key_name(table: &str, column: &str) -> String {
    format!("fk_{table}_{column}")
}

fn add_foreign_key_statement(
    from_table: &str,
    from_column: &str,
    to_table: &str,
    to_column: &str,
) -> ForeignKeyCreateStatement {
    let mut stmt = SqForeignKey::create();
    stmt.name(foreign_key_name(from_table, from_column))
        .from(Alias::new(from_table), Alias::new(from_column))
        .to(Alias::new(to_table), Alias::new(to_column));
    stmt
}

fn remove_foreign_key_statement(from_table: &str, from_column: &str) -> ForeignKeyDropStatement {
    let mut stmt = SqForeignKey::drop();
    stmt.name(foreign_key_name(from_table, from_column))
        .table(Alias::new(from_table));
    stmt
}

/// `add_foreign_key :from_table, :to_table` — constraint
/// `fk_<from_table>_<from_column>` linking `from_table.from_column` to
/// `to_table.to_column`.
///
/// Note: SQLite cannot add foreign keys via `ALTER TABLE`; define them inside
/// [`super::create_table`] there. This works on PostgreSQL and MySQL.
pub async fn add_foreign_key(
    manager: &SchemaManager<'_>,
    from_table: &str,
    from_column: &str,
    to_table: &str,
    to_column: &str,
) -> Result<(), DbErr> {
    add_foreign_key_with_opts(
        manager,
        from_table,
        from_column,
        to_table,
        to_column,
        CmdOpts::default(),
    )
    .await
}

/// Like [`add_foreign_key`], with explicit [`CmdOpts`].
pub async fn add_foreign_key_with_opts(
    manager: &SchemaManager<'_>,
    from_table: &str,
    from_column: &str,
    to_table: &str,
    to_column: &str,
    opts: CmdOpts,
) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    match manager
        .get_connection()
        .execute(&add_foreign_key_statement(
            from_table,
            from_column,
            to_table,
            to_column,
        ))
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if opts.if_not_exists && opts::is_duplicate_foreign_key_err(backend, &e) => Ok(()),
        Err(e) => Err(e),
    }
}

/// `remove_foreign_key :from_table, column: :from_column`.
pub async fn remove_foreign_key(
    manager: &SchemaManager<'_>,
    from_table: &str,
    from_column: &str,
) -> Result<(), DbErr> {
    remove_foreign_key_with_opts(manager, from_table, from_column, CmdOpts::default()).await
}

/// Like [`remove_foreign_key`], with explicit [`CmdOpts`].
pub async fn remove_foreign_key_with_opts(
    manager: &SchemaManager<'_>,
    from_table: &str,
    from_column: &str,
    opts: CmdOpts,
) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    match manager
        .get_connection()
        .execute(&remove_foreign_key_statement(from_table, from_column))
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if opts.if_exists => {
            let msg = e.to_string().to_lowercase();
            let missing = match backend {
                crate::sea_orm::DatabaseBackend::MySql => {
                    msg.contains("1091") || msg.contains("check that column/key exists")
                }
                crate::sea_orm::DatabaseBackend::Postgres => msg.contains("does not exist"),
                crate::sea_orm::DatabaseBackend::Sqlite => msg.contains("no such"),
                _ => msg.contains("does not exist"),
            };
            if missing {
                Ok(())
            } else {
                Err(e)
            }
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    fn foreign_key_name_is_derived() {
        assert_eq!(foreign_key_name("posts", "user_id"), "fk_posts_user_id");
    }

    #[test]
    fn add_foreign_key_renders_constraint() {
        let sql = add_foreign_key_statement("posts", "user_id", "users", "id")
            .to_string(PostgresQueryBuilder);
        assert!(sql.contains("\"fk_posts_user_id\""));
        assert!(sql.contains("\"posts\""));
        assert!(sql.contains("\"users\""));
    }
}
