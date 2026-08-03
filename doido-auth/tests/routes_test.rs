//! Auth routes integration tests.

use doido_auth::routes::mount;
use doido_auth::testing::{create_test_user, init_test_auth, send, test_auth_config, TestUser};
use doido_model::testing::TestDb;
use http::StatusCode;

#[tokio::test]
async fn sign_in_creates_session_cookie() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "dana@example.com", "secret")
        .await
        .unwrap();

    let app = mount::<TestUser, _>(|_db, _email, _digest| {
        Box::pin(async { panic!("sign_up create should not run") })
    });

    let resp = send(
        app,
        "POST",
        "/users/sign_in",
        r#"{"email":"dana@example.com","password":"secret"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.set_cookie.is_some());
    assert!(resp.set_cookie.unwrap().contains("_doido_session"));
}

#[tokio::test]
async fn sign_up_registers_user() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();

    let app = mount::<TestUser, _>(|_db, email, digest| {
        Box::pin(async move {
            Ok(TestUser {
                id: 1,
                email,
                password_digest: digest,
            })
        })
    });

    let resp = send(
        app,
        "POST",
        "/users/sign_up",
        r#"{"email":"new@example.com","password":"secret"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED);
    assert!(resp.body.contains("new@example.com"));
}

#[tokio::test]
async fn sign_in_rejects_bad_password() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "eve@example.com", "secret")
        .await
        .unwrap();

    let app = mount::<TestUser, _>(|_db, _email, _digest| Box::pin(async { panic!("not called") }));

    let resp = send(
        app,
        "POST",
        "/users/sign_in",
        r#"{"email":"eve@example.com","password":"wrong"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sign_out_clears_session_cookie() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();

    let app = mount::<TestUser, _>(|_db, _email, _digest| Box::pin(async { panic!("not called") }));

    let resp = send(app, "DELETE", "/users/sign_out", "").await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.set_cookie.unwrap().contains("Max-Age=0"));
}

#[tokio::test]
async fn sign_in_with_jwt_returns_token_pair() {
    let db = TestDb::new().await.unwrap();
    let config = doido_auth::testing::test_jwt_auth_config("routes-jwt-secret");
    let _auth = init_test_auth(db.conn().clone(), config).await.unwrap();
    create_test_user(db.conn(), "jwt-user@example.com", "secret")
        .await
        .unwrap();

    let app = mount::<TestUser, _>(|_db, _email, _digest| Box::pin(async { panic!("not called") }));

    let resp = send(
        app,
        "POST",
        "/users/sign_in",
        r#"{"email":"jwt-user@example.com","password":"secret"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body.contains("access_token"));
    assert!(resp.set_cookie.is_none());
}

#[tokio::test]
async fn password_reset_routes_respond() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();

    let app = mount::<TestUser, _>(|_db, _email, _digest| Box::pin(async { panic!("not called") }));

    let new_form = send(app.clone(), "GET", "/users/password/new", "").await;
    assert_eq!(new_form.status, StatusCode::OK);

    let request_reset = send(
        app.clone(),
        "POST",
        "/users/password",
        r#"{"email":"x@example.com"}"#,
    )
    .await;
    assert_eq!(request_reset.status, StatusCode::ACCEPTED);

    let patch = send(app, "PATCH", "/users/password", r#"{"password":"new"}"#).await;
    assert_eq!(patch.status, StatusCode::OK);
}

#[tokio::test]
async fn sign_up_rejects_duplicate_email() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    create_test_user(db.conn(), "taken@example.com", "secret")
        .await
        .unwrap();

    let app = mount::<TestUser, _>(|_db, email, digest| {
        Box::pin(async move {
            Ok(TestUser {
                id: 2,
                email,
                password_digest: digest,
            })
        })
    });

    let resp = send(
        app,
        "POST",
        "/users/sign_up",
        r#"{"email":"taken@example.com","password":"secret"}"#,
    )
    .await;
    assert_eq!(resp.status, StatusCode::UNPROCESSABLE_ENTITY);
}
