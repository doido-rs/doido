//! `rememberable` module — remember cookie + strategy.

use doido_auth::config::AuthConfig;
use doido_auth::rememberable::{self, RememberStrategy, REMEMBER_COOKIE};
use doido_auth::strategy::AuthStrategy;
use doido_auth::testing::init_test_auth;
use doido_auth::AuthModule;
use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement};
use doido_model::testing::TestDb;

const CREATE_USERS: &str = "CREATE TABLE users (\
    id INTEGER PRIMARY KEY, email TEXT NOT NULL, password_digest TEXT, remember_created_at TEXT)";

async fn setup(db: &TestDb) {
    db.conn()
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            CREATE_USERS.to_string(),
        ))
        .await
        .unwrap();
    db.conn()
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO users (email, password_digest) VALUES ('a@b.com', 'x')".to_string(),
        ))
        .await
        .unwrap();
}

async fn remember_created_at(db: &TestDb) -> Option<String> {
    let row = db
        .conn()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT remember_created_at FROM users WHERE email='a@b.com'".to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get::<Option<String>>("", "remember_created_at")
        .unwrap()
}

#[tokio::test]
async fn records_and_forgets_remember_timestamp() {
    let db = TestDb::new().await.unwrap();
    setup(&db).await;
    let cfg = AuthConfig {
        modules: vec![
            AuthModule::DatabaseAuthenticatable,
            AuthModule::Rememberable,
        ],
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), cfg).await.unwrap();

    rememberable::record_remember(db.conn(), "a@b.com")
        .await
        .unwrap();
    assert!(remember_created_at(&db).await.is_some());

    rememberable::forget(db.conn(), "a@b.com").await.unwrap();
    assert!(remember_created_at(&db).await.is_none());
}

#[tokio::test]
async fn strategy_resolves_signed_remember_cookie() {
    let db = TestDb::new().await.unwrap();

    // Build a signed remember cookie the way the sessions controller does.
    let mut jar =
        doido_controller::CookieJar::from_header(None, doido_controller::secret::key_base());
    jar.set_signed_permanent(REMEMBER_COOKIE, rememberable::cookie_value(&5_i64), 100);
    let set_cookie = jar.to_set_cookie_headers().into_iter().next().unwrap();
    // "_doido_remember=<value>; Path=/; ..." → the "name=value" pair.
    let pair = set_cookie.split(';').next().unwrap().to_string();

    let parts = http::Request::builder()
        .header(http::header::COOKIE, pair)
        .body(())
        .unwrap()
        .into_parts()
        .0;

    let identity = RememberStrategy
        .authenticate(&parts, db.conn())
        .await
        .unwrap()
        .expect("identity from remember cookie");
    assert_eq!(identity.user_id, serde_json::json!(5));
}

#[tokio::test]
async fn strategy_returns_none_without_cookie() {
    let db = TestDb::new().await.unwrap();
    let parts = http::Request::builder().body(()).unwrap().into_parts().0;
    assert!(RememberStrategy
        .authenticate(&parts, db.conn())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn record_remember_noop_when_disabled() {
    let db = TestDb::new().await.unwrap();
    setup(&db).await;
    let cfg = AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable],
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), cfg).await.unwrap();

    rememberable::record_remember(db.conn(), "a@b.com")
        .await
        .unwrap();
    assert!(remember_created_at(&db).await.is_none());
}
