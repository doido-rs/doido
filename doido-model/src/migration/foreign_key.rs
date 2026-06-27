//! Foreign-key operations: `add_foreign_key`, `remove_foreign_key`.

use sea_orm::sea_query::{
    Alias, ForeignKey as SqForeignKey, ForeignKeyCreateStatement, ForeignKeyDropStatement,
};
use sea_orm::{ConnectionTrait, DbErr};

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

/// Foreign-key migration operations.
pub struct ForeignKey;

impl ForeignKey {
    /// `add_foreign_key :from_table, :to_table` — constraint
    /// `fk_<from_table>_<from_column>` linking `from_table.from_column` to
    /// `to_table.to_column`.
    ///
    /// Note: SQLite cannot add foreign keys via `ALTER TABLE`; define them inside
    /// [`super::Table::create`] there. This works on PostgreSQL and MySQL.
    pub async fn add<C: ConnectionTrait>(
        db: &C,
        from_table: &str,
        from_column: &str,
        to_table: &str,
        to_column: &str,
    ) -> Result<(), DbErr> {
        db.execute(&add_foreign_key_statement(
            from_table,
            from_column,
            to_table,
            to_column,
        ))
        .await
        .map(|_| ())
    }

    /// `remove_foreign_key :from_table, column: :from_column`.
    pub async fn remove<C: ConnectionTrait>(
        db: &C,
        from_table: &str,
        from_column: &str,
    ) -> Result<(), DbErr> {
        db.execute(&remove_foreign_key_statement(from_table, from_column))
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::sea_query::PostgresQueryBuilder;

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
