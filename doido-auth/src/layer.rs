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

/// Stage the authenticated user for views (Rails `current_user` helper analogue).
///
/// Assigns the full serialized user under `current_user` and a `signed_in`
/// boolean onto the controller [`Context`](doido_controller::Context), so every
/// template rendered on this request can use e.g.
/// `{% if signed_in %}{{ current_user.email }}{% endif %}`. When no user is
/// authenticated it only sets `signed_in = false`. Designed to be wired as a
/// `#[before_action]` — generated controllers do this automatically.
///
/// The user model must derive `Serialize`; the generated user entity skips
/// `password_digest` so the hash never reaches the template context.
pub async fn assign_current_user<U>(ctx: &mut doido_controller::Context)
where
    U: crate::user::AuthUser + serde::Serialize,
{
    let loaded = current_user::<U>(ctx.request_parts()).await;
    match loaded {
        Ok(user) => {
            ctx.assign("current_user", &user);
            ctx.assign("signed_in", true);
        }
        Err(_) => {
            ctx.assign("signed_in", false);
        }
    }
}
