//! Generic user persistence contract — replaces the legacy [`AuthUser`] surface.

pub mod action;
pub mod identity;
pub mod plan;
pub mod row;
pub mod sql;
pub mod store;

#[cfg(feature = "auth-redis")]
pub mod redis_store;

pub use action::{UserPatchBuilder, UserStoreAction};
pub use identity::AuthIdentity;
pub use plan::{FetchContribution, FetchPlan, UserField};
pub use row::{SignInRow, UserRow};
pub use store::{build_user_store, SeaOrmUserStore, UserStore};

use crate::error::AuthError;
use doido_core::Result;
use doido_model::password::HasSecurePassword;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;

/// Authenticated subject + persistence commands for any backing store.
pub trait UserModel: Clone + Send + Sync + 'static {
    type Id: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + Serialize
        + DeserializeOwned
        + Into<serde_json::Value>
        + 'static;

    fn from_row(row: UserRow) -> std::result::Result<Self, AuthError>;
    fn id(&self) -> Self::Id;
    fn email(&self) -> &str;
    fn password_digest(&self) -> Option<&str>;

    fn find_by_email(
        store: &dyn UserStore,
        email: &str,
        plan: &FetchPlan,
    ) -> impl Future<Output = Result<Option<Self>>> + Send;

    fn find_by_id(
        store: &dyn UserStore,
        id: Self::Id,
        plan: &FetchPlan,
    ) -> impl Future<Output = Result<Option<Self>>> + Send;

    fn insert(store: &dyn UserStore, row: UserRow) -> impl Future<Output = Result<Self>> + Send;
}

pub fn authenticate_password<U: UserModel + HasSecurePassword>(user: &U, password: &str) -> bool {
    user.authenticate(password)
}

/// Register a new user from email + bcrypt digest (built-in registrations controller).
pub trait RegisterableUserModel: UserModel + HasSecurePassword {
    fn register(
        store: &dyn UserStore,
        email: String,
        password_digest: String,
    ) -> impl Future<Output = Result<Self>> + Send;
}

/// Helper: fetch + map row — typical `UserModel::find_by_email` body.
pub async fn find_by_email_mapped<U: UserModel>(
    store: &dyn UserStore,
    email: &str,
    plan: &FetchPlan,
) -> Result<Option<U>, AuthError> {
    let row = store.fetch_by_email(email, plan).await?;
    row.map(U::from_row).transpose()
}

pub async fn find_by_id_mapped<U: UserModel>(
    store: &dyn UserStore,
    id: U::Id,
    plan: &FetchPlan,
) -> Result<Option<U>, AuthError> {
    let sea_id = id_to_sea_value(&id)?;
    let row = store.fetch_by_id(sea_id, plan).await?;
    row.map(U::from_row).transpose()
}

pub fn id_to_sea_value<Id: Serialize>(id: &Id) -> Result<doido_model::sea_orm::Value, AuthError> {
    let json = serde_json::to_value(id).map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(json_to_sea_value(json))
}

fn json_to_sea_value(json: serde_json::Value) -> doido_model::sea_orm::Value {
    use doido_model::sea_orm::Value;
    match json {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::BigInt(Some(i))
            } else if let Some(u) = n.as_u64() {
                Value::BigUnsigned(Some(u))
            } else {
                Value::String(Some(n.to_string()))
            }
        }
        serde_json::Value::String(s) => Value::String(Some(s)),
        other => Value::Json(Some(Box::new(other))),
    }
}
