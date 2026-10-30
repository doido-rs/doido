//! HTTP Basic Auth for the jobs dashboard when credentials are configured.

use crate::config::WebUiConfig;
use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
    middleware::Next,
};
use base64::Engine;
use std::sync::Arc;

pub fn credentials_configured(webui: &WebUiConfig) -> bool {
    webui.username.as_ref().is_some_and(|u| !u.is_empty())
        && webui.password.as_ref().is_some_and(|p| !p.is_empty())
}

pub async fn basic_auth_middleware(
    axum::extract::State(expected): axum::extract::State<Arc<BasicAuthCredentials>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| parse_basic(v, &expected.username, &expected.password));
    if authorized.is_some() {
        next.run(request).await
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::WWW_AUTHENTICATE, "Basic realm=\"Doido Jobs\"")
            .body(Body::from("Unauthorized"))
            .expect("valid 401")
    }
}

#[derive(Clone)]
pub struct BasicAuthCredentials {
    pub username: String,
    pub password: String,
}

fn parse_basic(header: &str, user: &str, pass: &str) -> Option<()> {
    let encoded = header.strip_prefix("Basic ")?.trim();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let pair = String::from_utf8(decoded).ok()?;
    let (u, p) = pair.split_once(':')?;
    if subtle::ConstantTimeEq::ct_eq(u.as_bytes(), user.as_bytes()).into()
        && subtle::ConstantTimeEq::ct_eq(p.as_bytes(), pass.as_bytes()).into()
    {
        Some(())
    } else {
        None
    }
}
