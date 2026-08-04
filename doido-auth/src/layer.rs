//! Axum middleware that resolves auth identity via enabled strategies.

use crate::error::AuthError;
use crate::identity::AuthIdentity;
use crate::state::global;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Tower/axum layer function: consults strategies in config order and stores the
/// first resolved [`AuthIdentity`] in request extensions.
pub async fn auth_layer(req: Request<Body>, next: Next) -> Response {
    let state = match crate::state::try_global() {
        Some(s) => s,
        None => return next.run(req).await,
    };

    let (mut parts, body) = req.into_parts();
    for strategy in &state.strategies {
        match strategy.authenticate(&parts, &state.db).await {
            Ok(Some(identity)) => {
                parts.extensions.insert(identity);
                break;
            }
            Ok(None) => {}
            Err(_) => {}
        }
    }
    next.run(Request::from_parts(parts, body)).await
}

/// Read the authenticated identity from request extensions.
pub fn current_identity(parts: &http::request::Parts) -> Option<AuthIdentity> {
    crate::strategy::identity_from_parts(parts)
}

/// Load the current user model from extensions + DB.
pub async fn current_user<U: crate::user::AuthUser>(
    parts: &http::request::Parts,
) -> Result<U, AuthError> {
    let identity = crate::strategy::require_identity(parts)?;
    let id: U::Id = serde_json::from_value(identity.user_id.clone())
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    U::find_by_id(&global().db, id)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?
        .ok_or(AuthError::Unauthorized)
}
