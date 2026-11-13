use crate::error::AuthError;
use crate::user_model::plan::{FetchPlan, UserField};
use crate::user_model::row::UserRow;
use doido_model::sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement, Value,
};

pub fn detect_backend(db: &DatabaseConnection) -> DbBackend {
    db.get_database_backend()
}

pub async fn fetch_by_email(
    db: &DatabaseConnection,
    backend: DbBackend,
    email: &str,
    plan: &FetchPlan,
) -> Result<Option<UserRow>, AuthError> {
    let sql = match backend {
        DbBackend::Sqlite => super::sqlite::select_by_email(plan),
        DbBackend::Postgres => super::postgres::select_by_email(plan),
        DbBackend::MySql => super::mysql::select_by_email(plan),
        _ => super::sqlite::select_by_email(plan),
    };
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            sql,
            [Value::from(email.to_string())],
        ))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(row.map(|r| row_from_query(r, plan)))
}

pub async fn fetch_by_field(
    db: &DatabaseConnection,
    backend: DbBackend,
    field: UserField,
    value: &str,
    plan: &FetchPlan,
) -> Result<Option<UserRow>, AuthError> {
    if field == UserField::Email {
        return fetch_by_email(db, backend, value, plan).await;
    }
    let col = field.column_name();
    let sql = match backend {
        DbBackend::Sqlite => super::sqlite::select_by_column(plan, col),
        DbBackend::Postgres => super::postgres::select_by_column(plan, col),
        DbBackend::MySql => super::mysql::select_by_column(plan, col),
        _ => super::sqlite::select_by_column(plan, col),
    };
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            sql,
            [Value::from(value.to_string())],
        ))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(row.map(|r| row_from_query(r, plan)))
}

pub async fn fetch_by_id(
    db: &DatabaseConnection,
    backend: DbBackend,
    id: Value,
    plan: &FetchPlan,
) -> Result<Option<UserRow>, AuthError> {
    let sql = match backend {
        DbBackend::Sqlite => super::sqlite::select_by_id(plan),
        DbBackend::Postgres => super::postgres::select_by_id(plan),
        DbBackend::MySql => super::mysql::select_by_id(plan),
        _ => super::sqlite::select_by_id(plan),
    };
    let row = db
        .query_one_raw(Statement::from_sql_and_values(backend, sql, [id]))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(row.map(|r| row_from_query(r, plan)))
}

fn row_from_query(row: QueryResult, plan: &FetchPlan) -> UserRow {
    let mut out = UserRow::default();
    for field in &plan.fields {
        let col = field.column_name();
        if field == &UserField::FailedAttempts || field == &UserField::SignInCount {
            if let Ok(v) = row.try_get::<i32>("", col) {
                out.set_from_column(*field, Some(v.to_string()));
            }
            continue;
        }
        if field == &UserField::TwoFactorEnabled {
            if let Ok(v) = row.try_get::<bool>("", col) {
                out.set_from_column(*field, Some(v.to_string()));
            } else if let Ok(v) = row.try_get::<i32>("", col) {
                out.set_from_column(*field, Some((v != 0).to_string()));
            }
            continue;
        }
        let value: Option<String> = row.try_get::<Option<String>>("", col).ok().flatten();
        if field == &UserField::Id {
            if let Ok(id) = row.try_get::<i64>("", col) {
                out.set_id_json(serde_json::json!(id));
            } else if let Ok(id) = row.try_get::<i32>("", col) {
                out.set_id_json(serde_json::json!(id));
            } else if let Some(s) = value.clone() {
                out.set_id_json(serde_json::Value::String(s));
            }
        } else {
            out.set_from_column(*field, value);
        }
    }
    out
}
