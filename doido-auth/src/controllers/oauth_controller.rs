//! Default OAuth redirect/callback controller.

use crate::middleware::omniauthable;
use crate::middleware::omniauthable::OAuthTokenResponse;
use doido_auth_macros::auth_controller;
use doido_controller::axum::http::StatusCode;
use doido_controller::axum::response::{IntoResponse, Redirect, Response};
use doido_core::Result;
use serde::Deserialize;

/// Default OAuth controller for [`auth_routes!`](crate::auth_routes).
pub struct AuthOauth;

#[derive(Debug, Deserialize)]
struct OAuthCallbackQuery {
    code: String,
    #[allow(dead_code)]
    state: Option<String>,
}

#[auth_controller]
impl AuthOauth {
    /// GET `/auth/{provider}` — start the OAuth authorization flow.
    pub async fn authorize(ctx: doido_controller::Context) -> Result<Response> {
        let provider = ctx.param("provider").unwrap_or("unknown");
        let state = uuid::Uuid::new_v4().to_string();
        let url = omniauthable::authorize_url(provider, &state)?;
        Ok(Redirect::temporary(&url).into_response())
    }

    /// GET `/auth/{provider}/callback` — OAuth provider callback.
    pub async fn callback(ctx: doido_controller::Context) -> Result<Response> {
        let provider = ctx.param("provider").unwrap_or("unknown");
        let query: OAuthCallbackQuery = ctx.params()?;
        let tokens: OAuthTokenResponse = omniauthable::exchange_code(provider, &query.code)?;
        Ok((StatusCode::OK, doido_controller::axum::Json(tokens)).into_response())
    }
}
