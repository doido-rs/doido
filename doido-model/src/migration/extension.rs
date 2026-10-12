//! PostgreSQL extension helpers (`CREATE EXTENSION`).

use crate::sea_orm::{ConnectionTrait, DatabaseBackend, DbErr};
use crate::sea_orm_migration::SchemaManager;

/// `CREATE EXTENSION IF NOT EXISTS "name"` on PostgreSQL; no-op on other backends.
pub async fn enable_extension(manager: &SchemaManager<'_>, name: &str) -> Result<(), DbErr> {
    let backend = manager.get_database_backend();
    if backend != DatabaseBackend::Postgres {
        doido_core::tracing::debug!(
            extension = name,
            ?backend,
            "enable_extension skipped (PostgreSQL only)"
        );
        return Ok(());
    }

    let escaped = name.replace('"', "\"\"");
    let sql = format!(r#"CREATE EXTENSION IF NOT EXISTS "{escaped}""#);
    manager
        .get_connection()
        .execute_unprepared(&sql)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    #[test]
    fn enable_extension_sql_quotes_name() {
        let escaped = "pgcrypto".replace('"', "\"\"");
        let sql = format!(r#"CREATE EXTENSION IF NOT EXISTS "{escaped}""#);
        assert_eq!(sql, r#"CREATE EXTENSION IF NOT EXISTS "pgcrypto""#);
    }
}
