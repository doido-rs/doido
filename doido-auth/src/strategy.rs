//! Pluggable authentication strategies.

use crate::error::AuthError;
use crate::identity::AuthIdentity;
use async_trait::async_trait;
use doido_core::Result;
use doido_model::sea_orm::DatabaseConnection;
use http::request::Parts;

/// A pluggable auth backend (cookie session, JWT bearer, LDAP, …).
#[async_trait]
pub trait AuthStrategy: Send + Sync {
    fn name(&self) -> &str;

    async fn authenticate(
        &self,
        parts: &Parts,
        db: &DatabaseConnection,
    ) -> Result<Option<AuthIdentity>>;
}

/// Read the resolved identity from request extensions, if any.
pub fn identity_from_parts(parts: &Parts) -> Option<AuthIdentity> {
    parts.extensions.get::<AuthIdentity>().cloned()
}

/// Read the resolved identity or return [`AuthError::Unauthorized`].
pub fn require_identity(parts: &Parts) -> Result<AuthIdentity, AuthError> {
    identity_from_parts(parts).ok_or(AuthError::Unauthorized)
}
