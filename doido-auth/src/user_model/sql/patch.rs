use crate::error::AuthError;
use crate::user_model::action::{FieldChange, UserLocator, UserStoreAction};
use doido_model::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

pub async fn apply(
    db: &DatabaseConnection,
    backend: DbBackend,
    action: &UserStoreAction,
) -> Result<(), AuthError> {
    match action {
        UserStoreAction::Patch {
            locator,
            changes,
            add_indexes: _,
            remove_indexes: _,
        } => {
            let (sql, values) = build_patch_sql(locator, changes);
            db.execute_raw(Statement::from_sql_and_values(backend, &sql, values))
                .await
                .map_err(|e| AuthError::Internal(e.to_string()))?;
        }
        UserStoreAction::RecordSuccessfulSignIn {
            email,
            signed_in_at,
            ip,
        } => {
            let sql = "UPDATE users SET \
                sign_in_count = sign_in_count + 1, \
                last_sign_in_at = current_sign_in_at, \
                current_sign_in_at = ?, \
                last_sign_in_ip = current_sign_in_ip, \
                current_sign_in_ip = ? \
             WHERE email = ?";
            db.execute_raw(Statement::from_sql_and_values(
                backend,
                sql,
                [
                    Value::from(signed_in_at.clone()),
                    Value::from(ip.clone()),
                    Value::from(email.clone()),
                ],
            ))
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        }
    }
    Ok(())
}

fn build_patch_sql(locator: &UserLocator, changes: &[FieldChange]) -> (String, Vec<Value>) {
    let mut sets = Vec::new();
    let mut values = Vec::new();
    for change in changes {
        match change {
            FieldChange::Set { field, value } => {
                sets.push(format!("{} = ?", field.column_name()));
                values.push(Value::from(value.clone()));
            }
            FieldChange::Clear { field } => {
                sets.push(format!("{} = NULL", field.column_name()));
            }
            FieldChange::Increment { field, by } => {
                let col = field.column_name();
                sets.push(format!("{col} = {col} + ?"));
                values.push(Value::from(*by));
            }
        }
    }
    let where_clause = match locator {
        UserLocator::ByEmail(email) => {
            values.push(Value::from(email.clone()));
            "email = ?".to_string()
        }
        UserLocator::ByField { field, value } => {
            values.push(Value::from(value.clone()));
            format!("{} = ?", field.column_name())
        }
    };
    let sql = format!(
        "UPDATE users SET {} WHERE {}",
        sets.join(", "),
        where_clause
    );
    (sql, values)
}
