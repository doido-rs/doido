//! Pre-built auth routes (sessions, registration, passwords, OAuth).

use crate::error::AuthError;
use crate::handlers::{authenticate, register_user, sign_in_with_session};
use crate::oauth::get_provider;
use crate::session::{clear_session_cookie, encode_session_cookie, SESSION_COOKIE};
use crate::state::global;
use crate::user::AuthUser;
use axum::extract::{Path, Query};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use doido_controller::session::EncryptedCookieSessionStore;
use doido_model::password::HasSecurePassword;
use doido_model::sea_orm::DatabaseConnection;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Mount pre-built auth routes for user type `U`.
///
/// Apps pass a `create` closure that persists a newly registered user.
pub fn mount<U, F>(create: F) -> Router
where
    U: AuthUser + HasSecurePassword + Send + Sync + 'static,
    U::Id: serde::Serialize + DeserializeOwned,
    F: Fn(DatabaseConnection, String, String) -> BoxFuture<'static, doido_core::Result<U>>
        + Clone
        + Send
        + Sync
        + 'static,
{
    let routes = global().config.routes.clone();
    let sign_in_path = routes.sign_in_path();
    let sign_out_path = routes.sign_out_path();
    let sign_up_path = routes.sign_up_path();
    let password_path = routes.password_path();

    Router::new()
        .route(
            &sign_in_path,
            post({
                move |headers: HeaderMap, Json(body): Json<SignInBody>| {
                    sign_in_handler::<U>(headers, body)
                }
            }),
        )
        .route(&sign_out_path, delete(sign_out_handler))
        .route(
            &sign_up_path,
            post({
                move |Json(body): Json<SignUpBody>| sign_up_handler::<U, _>(body, create.clone())
            }),
        )
        .route(
            &format!("{password_path}/new"),
            get(|| async { StatusCode::OK }),
        )
        .route(
            &password_path,
            post(|| async {
                (
                    StatusCode::ACCEPTED,
                    Json(json!({"status":"reset_email_sent"})),
                )
            }),
        )
        .route(
            &password_path,
            patch(|| async { (StatusCode::OK, Json(json!({"status":"password_updated"}))) }),
        )
        .route("/auth/{provider}", get(oauth_redirect))
        .route("/auth/{provider}/callback", get(oauth_callback))
}

#[derive(Debug, Deserialize)]
struct SignInBody {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct SignUpBody {
    email: String,
    password: String,
}

async fn sign_in_handler<U>(headers: HeaderMap, body: SignInBody) -> Result<Response, AuthError>
where
    U: AuthUser + HasSecurePassword,
    U::Id: serde::Serialize,
{
    let state = global();
    let user = authenticate::<U>(&state.db, &body.email, &body.password).await?;
    let user_id =
        serde_json::to_value(user.id()).map_err(|e| AuthError::Internal(e.to_string()))?;

    if state.config.strategies.iter().any(|s| s == "jwt") {
        if let Some(jwt) = &state.jwt {
            let tokens = jwt.issue_tokens(&user_id)?;
            return Ok((StatusCode::OK, Json(tokens)).into_response());
        }
    }

    let mut session = session_from_headers(&headers);
    sign_in_with_session(&mut session, &user);
    let store = EncryptedCookieSessionStore::new(doido_controller::secret::key_base());
    let cookie = encode_session_cookie(&store, &session);
    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(json!({"status":"signed_in"})),
    )
        .into_response())
}

async fn sign_out_handler(_headers: HeaderMap) -> Response {
    let cookie = clear_session_cookie();
    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(json!({"status":"signed_out"})),
    )
        .into_response()
}

async fn sign_up_handler<U, F>(body: SignUpBody, create: F) -> Result<Response, AuthError>
where
    U: AuthUser + HasSecurePassword,
    F: Fn(DatabaseConnection, String, String) -> BoxFuture<'static, doido_core::Result<U>> + Clone,
{
    let state = global();
    let db = state.db.clone();
    let user = register_user::<U, _, _>(&state.db, &body.email, &body.password, |email, digest| {
        let create = create.clone();
        let db = db.clone();
        async move {
            create(db, email, digest)
                .await
                .map_err(|e| doido_core::anyhow::anyhow!(e.to_string()))
        }
    })
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({"status":"registered","email": user.email()})),
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
struct OAuthCallbackQuery {
    code: String,
    #[allow(dead_code)]
    state: Option<String>,
}

async fn oauth_redirect(Path(provider): Path<String>) -> Result<Response, AuthError> {
    let state = global();
    let oauth = state
        .oauth
        .get(&provider)
        .cloned()
        .or_else(|| get_provider(&provider))
        .ok_or_else(|| AuthError::OAuth(format!("unknown provider {provider}")))?;
    let url = oauth.authorize_url(&uuid::Uuid::new_v4().to_string())?;
    Ok(Redirect::temporary(&url).into_response())
}

async fn oauth_callback(
    Path(provider): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Response, AuthError> {
    let state = global();
    let oauth = state
        .oauth
        .get(&provider)
        .cloned()
        .or_else(|| get_provider(&provider))
        .ok_or_else(|| AuthError::OAuth(format!("unknown provider {provider}")))?;
    let tokens = oauth.exchange_code(&query.code)?;
    Ok((StatusCode::OK, Json(tokens)).into_response())
}

fn session_from_headers(headers: &HeaderMap) -> doido_controller::session::Session {
    let store = EncryptedCookieSessionStore::default();
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|header| {
            header
                .split(';')
                .filter_map(|pair| pair.trim().split_once('='))
                .find(|(k, _)| *k == SESSION_COOKIE)
                .and_then(|(_, v)| store.decode(v))
        })
        .unwrap_or_default()
}
