//! Direct coverage of the `doido_auth::testing` helpers themselves
//! (`hash_test_password`, `jwt_for_user`, `session_for_user`, the store-backed
//! `find_by_id`/`find_by_email`, and `send_with_headers`).

use doido_auth::testing::{
    create_test_user, hash_test_password, init_test_auth, jwt_for_user, send_with_headers,
    session_for_user, test_auth_config, test_jwt_auth_config, TestUser,
};
use doido_auth::AuthUser;
use doido_model::testing::TestDb;
use http::StatusCode;

#[test]
fn pure_helpers_hash_jwt_and_session() {
    let digest = hash_test_password("hunter2");
    assert!(!digest.is_empty());
    assert_ne!(digest, "hunter2");

    let config = test_jwt_auth_config("test-secret");
    let jwt_cfg = config.jwt.as_ref().expect("jwt config present");
    let token = jwt_for_user(jwt_cfg, 7);
    assert!(!token.is_empty());
    // A JWT is three dot-separated segments.
    assert_eq!(token.split('.').count(), 3);

    let user = TestUser {
        id: 42,
        email: "sess@example.com".into(),
        password_digest: digest,
    };
    // Signing into a fresh session must not panic and yields a usable session.
    let _session = session_for_user(&user);
}

#[tokio::test]
async fn store_backed_find_by_id_and_email() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "lookup@example.com", "secret123")
        .await
        .unwrap();

    let by_email = TestUser::find_by_email(db.conn(), "lookup@example.com")
        .await
        .unwrap()
        .expect("user found by email");
    let by_id = TestUser::find_by_id(db.conn(), by_email.id)
        .await
        .unwrap()
        .expect("user found by id");
    assert_eq!(by_id.email, "lookup@example.com");
    assert!(TestUser::find_by_email(db.conn(), "absent@example.com")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn send_with_headers_forwards_extra_headers() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "hdr@example.com", "secret123")
        .await
        .unwrap();
    let app = doido_auth::routes! {
        auth_routes!(TestUser);
    };
    let resp = send_with_headers(
        app,
        "POST",
        "/users/sign_in",
        r#"{"email":"hdr@example.com","password":"secret123"}"#,
        &[("x-request-id", "abc-123")],
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
}
