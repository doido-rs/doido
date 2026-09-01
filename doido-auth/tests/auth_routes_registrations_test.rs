//! Exercises the built-in `AuthRegistrations` controller through
//! `auth_routes!(TestUser)` (JSON mode), covering sign-up success and the
//! taken-email / confirmation-mismatch error branches (422).

use doido_auth::testing::{create_test_user, init_test_auth, send, test_auth_config, TestUser};
use doido_model::testing::TestDb;
use http::StatusCode;

fn app() -> doido_controller::axum::Router {
    doido_auth::routes! {
        auth_routes!(TestUser);
    }
}

#[tokio::test]
async fn sign_up_creates_account_and_signs_in() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();

    let resp = send(
        app(),
        "POST",
        "/users/sign_up",
        r#"{"email":"fresh@example.com","password":"secret123"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("fresh@example.com"));
    assert!(resp.set_cookie.unwrap().contains("_doido_session"));
}

#[tokio::test]
async fn sign_up_rejects_taken_email_with_422() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "dup@example.com", "secret123")
        .await
        .unwrap();

    let resp = send(
        app(),
        "POST",
        "/users/sign_up",
        r#"{"email":"dup@example.com","password":"secret123"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn sign_up_rejects_password_confirmation_mismatch() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();

    let resp = send(
        app(),
        "POST",
        "/users/sign_up",
        r#"{"email":"mismatch@example.com","password":"secret123","password_confirmation":"nope"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::UNPROCESSABLE_ENTITY);
}
