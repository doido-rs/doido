//! `timeoutable` module — expire sessions older than `auth.timeout`.

use doido_auth::config::AuthConfig;
use doido_auth::session::{is_session_expired, SIGNED_IN_AT_KEY, USER_ID_KEY};
use doido_auth::testing::init_test_auth;
use doido_auth::AuthModule;
use doido_controller::session::Session;
use doido_model::testing::TestDb;

fn session_signed_in_at(ts: i64) -> Session {
    let mut s = Session::default();
    s.set(USER_ID_KEY, 1);
    s.set(SIGNED_IN_AT_KEY, ts);
    s
}

#[tokio::test]
async fn expires_sessions_older_than_timeout() {
    let db = TestDb::new().await.unwrap();
    let cfg = AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable, AuthModule::Timeoutable],
        timeout: 60,
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), cfg).await.unwrap();

    let now = chrono::Utc::now().timestamp();
    assert!(is_session_expired(&session_signed_in_at(now - 120)), "2m-old session should expire");
    assert!(!is_session_expired(&session_signed_in_at(now - 10)), "fresh session should not expire");
}

#[tokio::test]
async fn noop_when_module_disabled() {
    let db = TestDb::new().await.unwrap();
    let cfg = AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable],
        timeout: 1,
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), cfg).await.unwrap();

    let ancient = chrono::Utc::now().timestamp() - 100_000;
    assert!(!is_session_expired(&session_signed_in_at(ancient)));
}
