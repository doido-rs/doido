//! Index operations: `add_index`, `remove_index`.

use sea_orm::sea_query::{Alias, Index as SqIndex, IndexCreateStatement, IndexDropStatement};
use sea_orm::{ConnectionTrait, DbErr};

/// Default index name for a `(table, columns)` pair: `idx_<table>_<col>_<col>`.
fn index_name(table: &str, columns: &[&str]) -> String {
    format!("idx_{table}_{}", columns.join("_"))
}

fn add_index_statement(table: &str, columns: &[&str]) -> IndexCreateStatement {
    let mut stmt = SqIndex::create();
    stmt.name(index_name(table, columns)).table(Alias::new(table));
    for col in columns {
        stmt.col(Alias::new(*col));
    }
    stmt
}

fn remove_index_statement(table: &str, columns: &[&str]) -> IndexDropStatement {
    let mut stmt = SqIndex::drop();
    stmt.name(index_name(table, columns)).table(Alias::new(table));
    stmt
}

/// Index migration operations.
pub struct Index;

impl Index {
    /// `add_index :table, [columns]` — index named `idx_<table>_<cols>`.
    pub async fn add<C: ConnectionTrait>(
        db: &C,
        table: &str,
        columns: &[&str],
    ) -> Result<(), DbErr> {
        db.execute(&add_index_statement(table, columns))
            .await
            .map(|_| ())
    }

    /// `remove_index :table, [columns]` — drops `idx_<table>_<cols>`.
    pub async fn remove<C: ConnectionTrait>(
        db: &C,
        table: &str,
        columns: &[&str],
    ) -> Result<(), DbErr> {
        db.execute(&remove_index_statement(table, columns))
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::sea_query::PostgresQueryBuilder;

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
