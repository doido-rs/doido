//! Extractor tests.

use doido_auth::extractors::{AuthToken, CurrentUser, MaybeUser, RequireAuth};
use doido_auth::identity::AuthIdentity;
use doido_auth::layer::auth_layer;
use doido_auth::testing::{
    create_test_user, init_test_auth, jwt_for_user, send, send_with_headers, test_auth_config,
    test_jwt_auth_config, TestUser,
};
use doido_auth::AuthError;
use doido_controller::axum::response::IntoResponse;
use doido_controller::axum::{routing::get, Router};
use doido_model::testing::TestDb;
use http::StatusCode;

async fn protected(CurrentUser(user): CurrentUser<TestUser>) -> String {
    format!("hello:{}", user.email)
}

async fn optional(MaybeUser(user): MaybeUser<TestUser>) -> String {
    match user {
        Some(u) => format!("user:{}", u.email),
        None => "guest".into(),
    }
}

async fn require_auth(RequireAuth(identity): RequireAuth) -> String {
    format!("auth:{}", identity.user_id)
}

async fn bearer(AuthToken(token): AuthToken) -> String {
    format!("token:{token}")
}

#[tokio::test]
async fn current_user_returns_401_without_identity() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let app = Router::new()
        .route("/me", get(protected))
        .layer(doido_controller::axum::middleware::from_fn(auth_layer));
    let resp = send(app, "GET", "/me", "").await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn maybe_user_returns_guest_without_identity() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let app = Router::new()
        .route("/home", get(optional))
        .layer(doido_controller::axum::middleware::from_fn(auth_layer));
    let resp = send(app, "GET", "/home", "").await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body, "guest");
}

#[tokio::test]
async fn current_user_loads_user_when_identity_present() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "carol@example.com", "secret")
        .await
        .unwrap();

    let app = Router::new().route("/me", get(protected)).layer(
        doido_controller::axum::middleware::from_fn(
            move |mut req: doido_controller::axum::http::Request<
                doido_controller::axum::body::Body,
            >,
                  next| {
                let user_id = user.id;
                async move {
                    req.extensions_mut().insert(AuthIdentity::new(user_id));
                    auth_layer(req, next).await
                }
            },
        ),
    );
    let resp = send(app, "GET", "/me", "").await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("carol@example.com"));
}

#[tokio::test]
async fn require_auth_extractor_succeeds_with_identity() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();

    let app = Router::new().route("/auth", get(require_auth)).layer(
        doido_controller::axum::middleware::from_fn(
            |mut req: doido_controller::axum::http::Request<doido_controller::axum::body::Body>,
             next| async move {
                req.extensions_mut().insert(AuthIdentity::new(7_i64));
                auth_layer(req, next).await
            },
        ),
    );
    let resp = send(app, "GET", "/auth", "").await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("7"));
}

#[tokio::test]
async fn auth_token_extractor_reads_bearer_header() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let app = Router::new().route("/token", get(bearer));
    let resp = send_with_headers(
        app,
        "GET",
        "/token",
        "",
        &[("Authorization", "Bearer my-jwt-token")],
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body, "token:my-jwt-token");
}

#[tokio::test]
async fn auth_token_extractor_rejects_missing_header() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let app = Router::new().route("/token", get(bearer));
    let resp = send(app, "GET", "/token", "").await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn maybe_user_returns_none_for_invalid_identity_id() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let app = Router::new().route("/home", get(optional)).layer(
        doido_controller::axum::middleware::from_fn(
            |mut req: doido_controller::axum::http::Request<doido_controller::axum::body::Body>,
             next| async move {
                req.extensions_mut()
                    .insert(AuthIdentity::new(serde_json::json!("not-a-number")));
                auth_layer(req, next).await
            },
        ),
    );
    let resp = send(app, "GET", "/home", "").await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body, "guest");
}

#[test]
fn auth_error_maps_to_http_status() {
    let resp = AuthError::EmailTaken.into_response();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let resp = AuthError::Internal("db".into()).into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn jwt_strategy_resolves_bearer_token() {
    let secret = "extractor-jwt-secret";
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_jwt_auth_config(secret))
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "jwt@example.com", "secret")
        .await
        .unwrap();
    let jwt_cfg = doido_auth::global().config.jwt.clone().unwrap();
    let token = jwt_for_user(&jwt_cfg, user.id);

    let app = Router::new()
        .route("/me", get(protected))
        .layer(doido_controller::axum::middleware::from_fn(auth_layer));
    let resp = send_with_headers(
        app,
        "GET",
        "/me",
        "",
        &[("Authorization", &format!("Bearer {token}"))],
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("jwt@example.com"));
}
