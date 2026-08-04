//! Handler unit tests (authenticate, sign-in/out, register).

use doido_auth::handlers::{authenticate, register_user, sign_in, sign_out};
use doido_auth::testing::{create_test_user, init_test_auth, test_auth_config, TestUser};
use doido_controller::axum::body::Body;
use doido_controller::Context;
use doido_model::testing::TestDb;
use http::Request;

#[tokio::test]
async fn authenticate_succeeds_with_valid_credentials() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "auth@example.com", "secret")
        .await
        .unwrap();

    let user: TestUser = authenticate(db.conn(), "auth@example.com", "secret")
        .await
        .unwrap();
    assert_eq!(user.email, "auth@example.com");
}

#[tokio::test]
async fn authenticate_rejects_unknown_email() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();

    let err = authenticate::<TestUser>(db.conn(), "missing@example.com", "secret")
        .await
        .unwrap_err();
    assert!(matches!(err, doido_auth::AuthError::InvalidCredentials));
}

#[tokio::test]
async fn register_user_rejects_duplicate_email() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "dup@example.com", "secret")
        .await
        .unwrap();

    let err = register_user(
        db.conn(),
        "dup@example.com",
        "other",
        |email, digest| async move {
            Ok(TestUser {
                id: 99,
                email,
                password_digest: digest,
            })
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, doido_auth::AuthError::EmailTaken));
}

#[tokio::test]
async fn sign_in_and_sign_out_update_session() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "ctx@example.com", "secret")
        .await
        .unwrap();

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let mut ctx = Context::build(req).await;

    sign_in(&mut ctx, &user).unwrap();
    assert_eq!(ctx.session().get::<i64>("user_id"), Some(user.id));

    sign_out(&mut ctx).unwrap();
    assert!(ctx.session().get::<i64>("user_id").is_none());
}
