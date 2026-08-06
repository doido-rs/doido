//! Default OAuth redirect/callback controller.

use crate::error::AuthError;
use crate::oauth::get_provider;
use crate::state::global;
use doido_controller::axum::http::StatusCode;
use doido_controller::axum::response::{IntoResponse, Redirect, Response};
use doido_controller::controller;
use doido_controller::Context;
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

#[controller]
impl AuthOauth {
    /// GET `/auth/{provider}` — start the OAuth authorization flow.
    pub async fn authorize(ctx: Context) -> Result<Response> {
        let provider = ctx.param("provider").unwrap_or("unknown");
        let state = global();
        let oauth = state
            .oauth
            .get(provider)
            .cloned()
            .or_else(|| get_provider(provider))
            .ok_or_else(|| AuthError::OAuth(format!("unknown provider {provider}")))?;
        let url = oauth.authorize_url(&uuid::Uuid::new_v4().to_string())?;
        Ok(Redirect::temporary(&url).into_response())
    }

    /// GET `/auth/{provider}/callback` — OAuth provider callback.
    pub async fn callback(ctx: Context) -> Result<Response> {
        let provider = ctx.param("provider").unwrap_or("unknown").to_string();
        let query: OAuthCallbackQuery = ctx.params()?;
        let state = global();
        let oauth = state
            .oauth
            .get(&provider)
            .cloned()
            .or_else(|| get_provider(&provider))
            .ok_or_else(|| AuthError::OAuth(format!("unknown provider {provider}")))?;
        let tokens = oauth.exchange_code(&query.code)?;
        Ok((
            StatusCode::OK,
            doido_controller::axum::Json(tokens),
        )
            .into_response())
    }
}
