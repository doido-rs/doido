//! Session strategy tests.

use doido_auth::session::{
    encode_session_cookie, sign_in_session, sign_out_session, SessionStrategy, SESSION_COOKIE,
    USER_ID_KEY,
};
use doido_auth::strategy::AuthStrategy;
use doido_auth::testing::{create_test_user, init_test_auth, test_auth_config};
use doido_controller::session::{EncryptedCookieSessionStore, Session};
use doido_model::testing::TestDb;

#[tokio::test]
async fn sign_in_stores_user_id_in_session() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "alice@example.com", "secret")
        .await
        .unwrap();

    let mut session = Session::new();
    sign_in_session(&mut session, user.id);
    assert_eq!(session.get::<i64>(USER_ID_KEY), Some(user.id));
    sign_out_session(&mut session);
    assert!(session.get::<i64>(USER_ID_KEY).is_none());
}

#[tokio::test]
async fn session_strategy_reads_encrypted_cookie() {
    let db = TestDb::new().await.unwrap();
    let _auth = init_test_auth(db.conn().clone(), test_auth_config())
        .await
        .unwrap();
    let user = create_test_user(db.conn(), "bob@example.com", "secret")
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
    let parts = http::Request::builder()
        .header(http::header::COOKIE, format!("{SESSION_COOKIE}={raw}"))
        .uri("/")
        .body(())
        .unwrap()
        .into_parts()
        .0;

    let strategy = SessionStrategy::default_dev();
    let identity = strategy
        .authenticate(&parts, db.conn())
        .await
        .unwrap()
        .expect("identity");
    assert_eq!(identity.user_id, serde_json::json!(user.id));
}

#[tokio::test]
async fn tampered_session_cookie_is_rejected() {
    let db = TestDb::new().await.unwrap();
    let strategy = SessionStrategy::default_dev();
    let parts = http::Request::builder()
        .header(http::header::COOKIE, "_doido_session=not-valid")
        .uri("/")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    let identity = strategy.authenticate(&parts, db.conn()).await.unwrap();
    assert!(identity.is_none());
}
