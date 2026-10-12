//! Data migrations via SeaQuery (`UPDATE` / `DELETE`) — no hand-written SQL strings in apps.

use crate::sea_orm::sea_query::{Alias, Query, SimpleExpr};
use crate::sea_orm::{ConnectionTrait, DbErr};
use crate::sea_orm_migration::SchemaManager;

/// `UPDATE target SET … FROM from_table WHERE join_on` (PostgreSQL-style `UPDATE … FROM`).
pub async fn update_from(
    manager: &SchemaManager<'_>,
    target_table: &str,
    set: &[(&str, SimpleExpr)],
    from_table: &str,
    join_on: SimpleExpr,
) -> Result<(), DbErr> {
    let mut stmt = Query::update();
    stmt.table(Alias::new(target_table));
    for (col, value) in set {
        stmt.value(Alias::new(*col), value.clone());
    }
    stmt.from(Alias::new(from_table));
    stmt.and_where(join_on);
    manager.get_connection().execute(&stmt).await.map(|_| ())
}

/// `UPDATE table SET … WHERE filter`.
pub async fn update_where(
    manager: &SchemaManager<'_>,
    table: &str,
    set: &[(&str, SimpleExpr)],
    filter: SimpleExpr,
) -> Result<(), DbErr> {
    let mut stmt = Query::update();
    stmt.table(Alias::new(table));
    for (col, value) in set {
        stmt.value(Alias::new(*col), value.clone());
    }
    stmt.and_where(filter);
    manager.get_connection().execute(&stmt).await.map(|_| ())
}

/// `DELETE FROM table WHERE filter`.
pub async fn delete_where(
    manager: &SchemaManager<'_>,
    table: &str,
    filter: SimpleExpr,
) -> Result<(), DbErr> {
    let mut stmt = Query::delete();
    stmt.from_table(Alias::new(table));
    stmt.cond_where(filter);
    manager.get_connection().execute(&stmt).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::sea_query::{Expr, ExprTrait, PostgresQueryBuilder, QueryStatementWriter as _};

    #[test]
    fn update_from_renders_postgres_from_clause() {
        let mut stmt = Query::update();
        stmt.table(Alias::new("patients"));
        stmt.value(
            Alias::new("identity_document"),
            Expr::col((Alias::new("users"), Alias::new("identity_document"))),
        );
        stmt.from(Alias::new("users"));
        stmt.and_where(
            Expr::col((Alias::new("patients"), Alias::new("user_id")))
                .equals((Alias::new("users"), Alias::new("id"))),
        );
        let sql = stmt.to_string(PostgresQueryBuilder);
        assert!(sql.contains("UPDATE \"patients\""));
        assert!(sql.contains("FROM \"users\""));
    }

    #[test]
    fn delete_where_renders_filter() {
        let mut stmt = Query::delete();
        stmt.from_table(Alias::new("specialties"));
        stmt.cond_where(Expr::col(Alias::new("company_id")).is_null());
        let sql = stmt.to_string(PostgresQueryBuilder);
        assert!(sql.contains("DELETE FROM \"specialties\""));
        assert!(sql.contains("company_id"));
    }
}
