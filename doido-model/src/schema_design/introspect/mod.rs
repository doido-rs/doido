//! Database introspection adapters.

#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;

use doido_core::Result;

use super::model::SchemaDesign;

/// Default tables omitted from ER diagrams.
pub const DEFAULT_IGNORE_TABLES: &[&str] = &["seaql_migrations"];

/// Introspect a live database and build an engine-agnostic [`SchemaDesign`].
pub async fn introspect_from_url(
    url: &str,
    _database_schema: Option<&str>,
    ignore_tables: &[String],
) -> Result<SchemaDesign> {
    let parsed = url::Url::parse(url)
        .map_err(|e| doido_core::anyhow::anyhow!("invalid database url: {e}"))?;

    match parsed.scheme() {
        "sqlite" => {
            #[cfg(feature = "sqlite")]
            {
                sqlite::introspect(url, ignore_tables).await
            }
            #[cfg(not(feature = "sqlite"))]
            {
                Err(doido_core::anyhow::anyhow!("sqlite feature is off"))
            }
        }
        "postgres" | "postgresql" => {
            #[cfg(feature = "postgres")]
            {
                postgres::introspect(url, _database_schema, ignore_tables).await
            }
            #[cfg(not(feature = "postgres"))]
            {
                Err(doido_core::anyhow::anyhow!("postgres feature is off"))
            }
        }
        "mysql" => {
            #[cfg(feature = "mysql")]
            {
                mysql::introspect(url, ignore_tables).await
            }
            #[cfg(not(feature = "mysql"))]
            {
                Err(doido_core::anyhow::anyhow!("mysql feature is off"))
            }
        }
        other => Err(doido_core::anyhow::anyhow!("unsupported database scheme: {other}")),
    }
}

/// Merge caller-supplied ignore list with framework defaults.
pub fn resolve_ignore_tables(extra: &[String]) -> Vec<String> {
    let mut tables: Vec<String> = DEFAULT_IGNORE_TABLES
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    for t in extra {
        if !tables.contains(t) {
            tables.push(t.clone());
        }
    }
    tables
}
