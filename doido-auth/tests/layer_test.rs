//! Auth layer middleware tests.

use doido_auth::layer::{auth_layer, current_identity, current_user};
use doido_auth::session::{encode_session_cookie, sign_in_session, SESSION_COOKIE};
use doido_auth::testing::{
    create_test_user, init_test_auth, send, send_with_headers, test_auth_config, TestUser,
};
use doido_controller::axum::{routing::get, Router};
use doido_controller::session::{EncryptedCookieSessionStore, Session};
use doido_model::testing::TestDb;
use http::StatusCode;

async fn whoami() -> String {
    "ok".into()
}

#[tokio::test]
async fn auth_layer_passes_through_without_session_cookie() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let app = Router::new()
        .route("/ping", get(whoami))
        .layer(doido_controller::axum::middleware::from_fn(auth_layer));
    let resp = send(app, "GET", "/ping", "").await;
    assert_eq!(resp.status, StatusCode::OK);
}

#[tokio::test]
async fn auth_layer_resolves_identity_from_session_cookie() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "layer@example.com", "secret")
        .await
        .unwrap();

    let mut session = Session::new();
    sign_in_session(&mut session, user.id);
    let store = EncryptedCookieSessionStore::default();
    let cookie = encode_session_cookie(&store, &session);
    let raw = cookie
        .strip_prefix(&format!("{SESSION_COOKIE}="))
        .unwrap()
        .split(';')
        .next()
        .unwrap();

    let app =
        Router::new()
            .route(
                "/me",
                get(
                    |req: doido_controller::axum::http::Request<
                        doido_controller::axum::body::Body,
                    >| async move {
                        let (parts, _) = req.into_parts();
                        match current_identity(&parts) {
                            Some(id) => format!("id:{}", id.user_id),
                            None => "none".into(),
                        }
                    },
                ),
            )
            .layer(doido_controller::axum::middleware::from_fn(auth_layer));

    let resp = send_with_headers(
        app,
        "GET",
        "/me",
        "",
        &[("Cookie", &format!("{SESSION_COOKIE}={raw}"))],
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains(&user.id.to_string()));
}

#[tokio::test]
async fn current_user_loads_from_identity() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "load@example.com", "secret")
        .await
        .unwrap();

    let mut parts = http::Request::builder()
        .uri("/")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    parts
        .extensions
        .insert(doido_auth::identity::AuthIdentity::new(user.id));

    let loaded: TestUser = current_user(&parts).await.unwrap();
    assert_eq!(loaded.email, "load@example.com");
}
