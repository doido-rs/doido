//! Generic authenticated subject contract — apps implement this on their User model.

use doido_core::Result;
use doido_model::password::HasSecurePassword;
use doido_model::sea_orm::DatabaseConnection;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;

/// The contract your User SeaORM model implements (manually or via `#[auth_user]`).
pub trait AuthUser: Clone + Send + Sync + 'static {
    /// Primary key type (`i64`, `Uuid`, …).
    type Id: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + Serialize
        + DeserializeOwned
        + Into<serde_json::Value>
        + 'static;

    fn id(&self) -> Self::Id;
    fn email(&self) -> &str;
    fn password_digest(&self) -> Option<&str>;

    fn find_by_email(
        db: &DatabaseConnection,
        email: &str,
    ) -> impl Future<Output = Result<Option<Self>>> + Send;

    fn find_by_id(
        db: &DatabaseConnection,
        id: Self::Id,
    ) -> impl Future<Output = Result<Option<Self>>> + Send;
}

/// Verify `password` against `user`'s stored digest when present.
pub fn authenticate_password<U: AuthUser + HasSecurePassword>(user: &U, password: &str) -> bool {
    user.authenticate(password)
}
