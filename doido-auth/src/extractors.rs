//! Axum extractors for authenticated requests.

use crate::error::AuthError;
use crate::identity::AuthIdentity;
use crate::state::global;
use crate::strategy::{identity_from_parts, require_identity};
use crate::user::AuthUser;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use http::header;
use std::future::{ready, Future, Ready};

/// Requires an authenticated user loaded from the database.
pub struct CurrentUser<U>(pub U);

/// Optional authenticated user — never fails the request.
pub struct MaybeUser<U>(pub Option<U>);

/// Ensures some identity is present without loading the full user model.
pub struct RequireAuth(pub AuthIdentity);

/// Raw bearer token string from the `Authorization` header.
pub struct AuthToken(pub String);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            AuthError::Unauthorized | AuthError::InvalidCredentials | AuthError::InvalidToken => {
                StatusCode::UNAUTHORIZED
            }
            AuthError::EmailTaken | AuthError::Validation(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            AuthError::NotConfirmed | AuthError::AccountLocked => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

impl<S, U> FromRequestParts<S> for CurrentUser<U>
where
    S: Send + Sync,
    U: AuthUser,
{
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let identity_result = require_identity(parts);
        let db = global().db.clone();
        async move {
            let identity = identity_result?;
            let id: U::Id = serde_json::from_value(identity.user_id)
                .map_err(|e| AuthError::Internal(e.to_string()))?;
            U::find_by_id(&db, id)
                .await
                .map_err(|e| AuthError::Internal(e.to_string()))?
                .ok_or(AuthError::Unauthorized)
                .map(CurrentUser)
        }
    }
}

impl<S, U> FromRequestParts<S> for MaybeUser<U>
where
    S: Send + Sync,
    U: AuthUser,
{
    type Rejection = std::convert::Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let identity = identity_from_parts(parts);
        let db = global().db.clone();
        async move {
            let user = match identity {
                Some(identity) => {
                    let id: U::Id = match serde_json::from_value(identity.user_id) {
                        Ok(id) => id,
                        Err(_) => return Ok(MaybeUser(None)),
                    };
                    U::find_by_id(&db, id).await.ok().flatten()
                }
                None => None,
            };
            Ok(MaybeUser(user))
        }
    }
}

impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        ready(require_identity(parts).map(RequireAuth))
    }
}

impl<S> FromRequestParts<S> for AuthToken
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthError::InvalidToken)
            .and_then(|value| {
                value
                    .strip_prefix("Bearer ")
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .ok_or(AuthError::InvalidToken)
            });
        ready(token.map(AuthToken))
    }
}

// Re-export axum for generated apps (mirrors doido-controller pattern).
pub use axum;

// Silence unused import when compiling with minimal features.
type _ReadyCheck = Ready<Result<(), AuthError>>;
