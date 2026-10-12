//! Index operations: `add_index`, `create_index`, `drop_index`, …

use crate::sea_orm::sea_query::{
    Alias, ConditionalStatement, Expr, Index as SqIndex, IndexCreateStatement, IndexDropStatement,
    IndexOrder,
};
use crate::sea_orm::{ConnectionTrait, DbErr};
use crate::sea_orm_migration::SchemaManager;

pub use crate::sea_orm::sea_query::IndexOrder as IndexSort;

/// One column in a [`create_index`] definition, optionally sorted `DESC`.
#[derive(Debug, Clone)]
pub struct IndexColumn {
    pub name: String,
    pub order: Option<IndexOrder>,
}

impl IndexColumn {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            order: None,
        }
    }

    pub fn desc(mut self) -> Self {
        self.order = Some(IndexOrder::Desc);
        self
    }
}

/// Options for [`create_index`] (unique, custom name, partial index predicate, …).
#[derive(Debug, Clone, Default)]
pub struct IndexOpts {
    pub unique: bool,
    pub name: Option<String>,
    pub where_expr: Option<Expr>,
    pub nulls_not_distinct: bool,
}

/// Default index name for a `(table, columns)` pair: `idx_<table>_<col>_<col>`.
pub fn default_index_name(table: &str, column_names: &[&str]) -> String {
    format!("idx_{table}_{}", column_names.join("_"))
}

fn index_name_from_columns(table: &str, columns: &[IndexColumn], opts: &IndexOpts) -> String {
    opts.name.clone().unwrap_or_else(|| {
        let names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
        default_index_name(table, &names)
    })
}

pub fn create_index_statement(
    table: &str,
    columns: &[IndexColumn],
    opts: &IndexOpts,
) -> IndexCreateStatement {
    let mut stmt = SqIndex::create();
    stmt.name(index_name_from_columns(table, columns, opts))
        .table(Alias::new(table));
    if opts.unique {
        stmt.unique();
    }
    if opts.nulls_not_distinct {
        stmt.nulls_not_distinct();
    }
    for col in columns {
        match &col.order {
            Some(order) => {
                stmt.col((Alias::new(col.name.as_str()), order.clone()));
            }
            None => {
                stmt.col(Alias::new(col.name.as_str()));
            }
        }
    }
    if let Some(filter) = &opts.where_expr {
        stmt.and_where(filter.clone());
    }
    stmt
}

pub fn drop_index_statement(name: &str, table: Option<&str>) -> IndexDropStatement {
    let mut stmt = SqIndex::drop();
    stmt.name(name);
    if let Some(table) = table {
        stmt.table(Alias::new(table));
    }
    stmt
}

fn legacy_columns(columns: &[&str]) -> Vec<IndexColumn> {
    columns
        .iter()
        .map(|c| IndexColumn::new(*c))
        .collect()
}

/// `create_index` — full control over name, uniqueness, sort order, and partial indexes.
pub async fn create_index(
    manager: &SchemaManager<'_>,
    table: &str,
    columns: &[IndexColumn],
    opts: IndexOpts,
) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&create_index_statement(table, columns, &opts))
        .await
        .map(|_| ())
}

/// Drops an index by explicit name (optional table for backends that require it).
pub async fn drop_index(
    manager: &SchemaManager<'_>,
    name: &str,
    table: Option<&str>,
) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(&drop_index_statement(name, table))
        .await
        .map(|_| ())
}

/// `add_index :table, [columns]` — index named `idx_<table>_<cols>`.
pub async fn add_index(
    manager: &SchemaManager<'_>,
    table: &str,
    columns: &[&str],
) -> Result<(), DbErr> {
    create_index(
        manager,
        table,
        &legacy_columns(columns),
        IndexOpts::default(),
    )
    .await
}

/// `remove_index :table, [columns]` — drops `idx_<table>_<cols>`.
pub async fn remove_index(
    manager: &SchemaManager<'_>,
    table: &str,
    columns: &[&str],
) -> Result<(), DbErr> {
    let name = default_index_name(table, columns);
    drop_index(manager, &name, Some(table)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::sea_query::PostgresQueryBuilder;
    use crate::sea_orm::sea_query::{ExprTrait, SchemaStatementBuilder as _};

    #[test]
    fn index_name_is_derived_from_table_and_columns() {
        assert_eq!(default_index_name("users", &["email"]), "idx_users_email");
        assert_eq!(default_index_name("users", &["a", "b"]), "idx_users_a_b");
    }

    #[test]
    fn partial_unique_index_renders_where_clause() {
        let sql = create_index_statement(
            "company_users",
            &[IndexColumn::new("calendar_feed_token")],
            &IndexOpts {
                unique: true,
                name: Some("company_users_calendar_feed_token_key".into()),
                where_expr: Some(Expr::col(Alias::new("calendar_feed_token")).is_not_null()),
                ..Default::default()
            },
        )
        .to_string(PostgresQueryBuilder);
        assert!(sql.contains("company_users_calendar_feed_token_key"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("IS NOT NULL"));
    }

    #[test]
    fn desc_index_column_renders() {
        let sql = create_index_statement(
            "billing_invoices",
            &[IndexColumn::new("due_at").desc()],
            &IndexOpts::default(),
        )
        .to_string(PostgresQueryBuilder);
        assert!(sql.contains("DESC"));
    }
}
