//! Database creation, backing `doido db create`.
//!
//! SeaORM has no "create database" primitive, so this implements the common
//! per-backend approach:
//!   * **SQLite** — ensure the parent directory exists and open the file with
//!     `mode=rwc`, which creates it.
//!   * **PostgreSQL / MySQL** — connect to the server *without* selecting the
//!     target database, then issue `CREATE DATABASE`.

use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DbErr};

/// Creates the database named in `url`.
///
/// Returns `Ok(())` once the database exists. Callers may inspect the error for
/// an "already exists" message to treat re-creation as a no-op.
pub async fn create_database(url: &str) -> Result<(), DbErr> {
    if is_sqlite(url) {
        return create_sqlite(url).await;
    }

    let (server_url, db_name) = split_server_and_database(url)?;
    let db = Database::connect(server_url).await?;
    let backend = db.get_database_backend();
    let quoted = quote_identifier(backend, &db_name)?;
    db.execute_unprepared(&format!("CREATE DATABASE {quoted}"))
        .await?;
    Ok(())
}

fn is_sqlite(url: &str) -> bool {
    url.starts_with("sqlite:")
}

/// Creates a SQLite database file (and its parent directory) by opening it with
/// `mode=rwc`.
async fn create_sqlite(url: &str) -> Result<(), DbErr> {
    let raw = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))
        .unwrap_or(url);
    let path = raw.split('?').next().unwrap_or(raw);

    if !path.is_empty() && !path.contains(":memory:") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| DbErr::Custom(format!("failed to create directory: {e}")))?;
            }
        }
    }

    Database::connect(ensure_sqlite_rwc(url)).await?;
    Ok(())
}

/// Appends `mode=rwc` so SQLite creates the file when missing.
fn ensure_sqlite_rwc(url: &str) -> String {
    if url.contains("mode=") {
        url.to_string()
    } else if url.contains('?') {
        format!("{url}&mode=rwc")
    } else {
        format!("{url}?mode=rwc")
    }
}

/// Splits `url` into a server-level connection URL (no target database) and the
/// target database name.
fn split_server_and_database(url: &str) -> Result<(String, String), DbErr> {
    let mut parsed =
        url::Url::parse(url).map_err(|e| DbErr::Custom(format!("invalid database URL: {e}")))?;
    let db_name = parsed.path().trim_start_matches('/').to_string();
    if db_name.is_empty() {
        return Err(DbErr::Custom(
            "database URL is missing a database name".to_string(),
        ));
    }

    let scheme = parsed.scheme().to_string();
    if scheme.starts_with("postgres") {
        // PostgreSQL requires connecting to an existing database; use the
        // always-present `postgres` maintenance database.
        parsed.set_path("/postgres");
    } else if scheme.starts_with("mysql") {
        // MySQL can connect without selecting a database.
        parsed.set_path("/");
    } else {
        return Err(DbErr::Custom(format!(
            "unsupported database scheme '{scheme}' for create"
        )));
    }

    Ok((parsed.to_string(), db_name))
}

/// Quotes a database identifier for the given backend.
fn quote_identifier(backend: DatabaseBackend, name: &str) -> Result<String, DbErr> {
    match backend {
        DatabaseBackend::Postgres => Ok(format!("\"{}\"", name.replace('"', "\"\""))),
        DatabaseBackend::MySql => Ok(format!("`{}`", name.replace('`', "``"))),
        DatabaseBackend::Sqlite => Err(DbErr::Custom(
            "SQLite databases are created as files, not via CREATE DATABASE".to_string(),
        )),
        other => Err(DbErr::Custom(format!(
            "unsupported database backend for create: {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_postgres_url_to_maintenance_db() {
        let (server, name) =
            split_server_and_database("postgres://user:pass@localhost:5432/my_app_development")
                .unwrap();
        assert_eq!(server, "postgres://user:pass@localhost:5432/postgres");
        assert_eq!(name, "my_app_development");
    }

    #[test]
    fn splits_mysql_url_dropping_database() {
        let (server, name) =
            split_server_and_database("mysql://root@localhost:3306/my_app_test").unwrap();
        assert_eq!(server, "mysql://root@localhost:3306/");
        assert_eq!(name, "my_app_test");
    }

    #[test]
    fn errors_when_database_name_missing() {
        assert!(split_server_and_database("postgres://localhost").is_err());
    }

    #[test]
    fn appends_rwc_to_sqlite_url() {
        assert_eq!(
            ensure_sqlite_rwc("sqlite://db/development.db"),
            "sqlite://db/development.db?mode=rwc"
        );
        assert_eq!(
            ensure_sqlite_rwc("sqlite://db/development.db?cache=shared"),
            "sqlite://db/development.db?cache=shared&mode=rwc"
        );
        assert_eq!(
            ensure_sqlite_rwc("sqlite://db/x.db?mode=rwc"),
            "sqlite://db/x.db?mode=rwc"
        );
    }

    #[test]
    fn quotes_identifiers_per_backend() {
        assert_eq!(
            quote_identifier(DatabaseBackend::Postgres, "app").unwrap(),
            "\"app\""
        );
        assert_eq!(
            quote_identifier(DatabaseBackend::MySql, "app").unwrap(),
            "`app`"
        );
    }
}
