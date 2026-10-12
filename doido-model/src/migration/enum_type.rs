//! PostgreSQL enum type DDL — kept in the framework so app migrations stay SQL-free.

use crate::sea_orm::{ConnectionTrait, DatabaseBackend, DbErr};
use crate::sea_orm_migration::SchemaManager;

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// `CREATE TYPE name AS ENUM (...)` on PostgreSQL; no-op elsewhere.
pub async fn create_enum_type(
    manager: &SchemaManager<'_>,
    name: &str,
    values: &[&str],
) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    if backend != DatabaseBackend::Postgres {
        doido_core::tracing::debug!(enum_type = name, ?backend, "create_enum_type skipped");
        return Ok(());
    }
    if values.is_empty() {
        return Err(DbErr::Custom(format!(
            "create_enum_type({name}): at least one enum value required"
        )));
    }
    let variants = values
        .iter()
        .map(|v| quote_literal(v))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("CREATE TYPE {} AS ENUM ({variants})", quote_ident(name));
    manager
        .get_connection()
        .execute_unprepared(&sql)
        .await
        .map(|_| ())
}

/// `ALTER TYPE name ADD VALUE 'value'` (PostgreSQL).
pub async fn add_enum_values(
    manager: &SchemaManager<'_>,
    name: &str,
    values: &[&str],
) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    if backend != DatabaseBackend::Postgres {
        return Ok(());
    }
    let conn = manager.get_connection();
    for value in values {
        let sql = format!(
            "ALTER TYPE {} ADD VALUE IF NOT EXISTS {}",
            quote_ident(name),
            quote_literal(value)
        );
        conn.execute_unprepared(&sql).await?;
    }
    Ok(())
}

/// `DROP TYPE IF EXISTS name` on PostgreSQL; no-op elsewhere.
pub async fn drop_enum_type(manager: &SchemaManager<'_>, name: &str) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    if backend != DatabaseBackend::Postgres {
        return Ok(());
    }
    let sql = format!("DROP TYPE IF EXISTS {}", quote_ident(name));
    manager
        .get_connection()
        .execute_unprepared(&sql)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_enum_type_sql() {
        let variants = ["a", "b"]
            .iter()
            .map(|v| quote_literal(v))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("CREATE TYPE {} AS ENUM ({variants})", quote_ident("status"));
        assert_eq!(sql, "CREATE TYPE \"status\" AS ENUM ('a', 'b')");
    }
}
