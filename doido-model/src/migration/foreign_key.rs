//! Foreign-key operations: `add_foreign_key`, `remove_foreign_key`, and named/action variants.

use crate::sea_orm::sea_query::{
    Alias, ForeignKey as SqForeignKey, ForeignKeyAction, ForeignKeyCreateStatement,
    ForeignKeyDropStatement,
};
use crate::sea_orm::{ConnectionTrait, DbErr};
use crate::sea_orm_migration::SchemaManager;

/// Options when adding a foreign key (custom constraint name, `ON DELETE` / `ON UPDATE`).
#[derive(Debug, Clone, Default)]
pub struct ForeignKeyOpts {
    pub name: Option<String>,
    pub on_delete: Option<ForeignKeyAction>,
    pub on_update: Option<ForeignKeyAction>,
}

/// Default foreign-key constraint name: `fk_<table>_<column>`.
pub fn default_foreign_key_name(table: &str, column: &str) -> String {
    format!("fk_{table}_{column}")
}

fn foreign_key_name(table: &str, column: &str, opts: &ForeignKeyOpts) -> String {
    opts.name
        .clone()
        .unwrap_or_else(|| default_foreign_key_name(table, column))
}

pub fn add_foreign_key_statement(
    from_table: &str,
    from_column: &str,
    to_table: &str,
    to_column: &str,
    opts: &ForeignKeyOpts,
) -> ForeignKeyCreateStatement {
    let mut stmt = SqForeignKey::create();
    stmt.name(foreign_key_name(from_table, from_column, opts))
        .from(Alias::new(from_table), Alias::new(from_column))
        .to(Alias::new(to_table), Alias::new(to_column));
    if let Some(action) = opts.on_delete {
        stmt.on_delete(action);
    }
    if let Some(action) = opts.on_update {
        stmt.on_update(action);
    }
    stmt
}

fn remove_foreign_key_statement(from_table: &str, constraint_name: &str) -> ForeignKeyDropStatement {
    let mut stmt = SqForeignKey::drop();
    stmt.name(constraint_name).table(Alias::new(from_table));
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
        ForeignKeyOpts::default(),
    )
    .await
}

/// Like [`add_foreign_key`] with explicit constraint naming and referential actions.
pub async fn add_foreign_key_with_opts(
    manager: &SchemaManager<'_>,
    from_table: &str,
    from_column: &str,
    to_table: &str,
    to_column: &str,
    opts: ForeignKeyOpts,
) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&add_foreign_key_statement(
            from_table,
            from_column,
            to_table,
            to_column,
            &opts,
        ))
        .await
        .map(|_| ())
}

/// `remove_foreign_key :from_table, column: :from_column` — drops `fk_<table>_<column>`.
pub async fn remove_foreign_key(
    manager: &SchemaManager<'_>,
    from_table: &str,
    from_column: &str,
) -> Result<(), DbErr> {
    remove_foreign_key_by_name(
        manager,
        from_table,
        &default_foreign_key_name(from_table, from_column),
    )
    .await
}

/// Drops a foreign-key constraint by its exact name.
pub async fn remove_foreign_key_by_name(
    manager: &SchemaManager<'_>,
    from_table: &str,
    constraint_name: &str,
) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&remove_foreign_key_statement(from_table, constraint_name))
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    fn foreign_key_name_is_derived() {
        assert_eq!(default_foreign_key_name("posts", "user_id"), "fk_posts_user_id");
    }

    #[test]
    fn add_foreign_key_renders_constraint() {
        let sql = add_foreign_key_statement("posts", "user_id", "users", "id", &ForeignKeyOpts {
            name: Some("fk-posts-user_id-to-users".into()),
            on_delete: Some(ForeignKeyAction::Cascade),
            ..Default::default()
        })
        .to_string(PostgresQueryBuilder);
        assert!(sql.contains("fk-posts-user_id-to-users"));
        assert!(sql.contains("ON DELETE CASCADE"));
    }
}
