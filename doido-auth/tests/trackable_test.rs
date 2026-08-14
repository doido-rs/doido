//! `trackable` module — sign-in statistics on the conventional `users` columns.

use doido_auth::config::AuthConfig;
use doido_auth::testing::init_test_auth;
use doido_auth::{trackable, AuthModule};
use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement};
use doido_model::testing::TestDb;

const CREATE_USERS: &str = "CREATE TABLE users (\
    id INTEGER PRIMARY KEY, email TEXT NOT NULL, password_digest TEXT, \
    sign_in_count INTEGER NOT NULL DEFAULT 0, \
    current_sign_in_at TEXT, last_sign_in_at TEXT, \
    current_sign_in_ip TEXT, last_sign_in_ip TEXT)";

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

async fn sign_in_count(db: &TestDb) -> i64 {
    let row = db
        .conn()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT sign_in_count, current_sign_in_ip, last_sign_in_ip FROM users WHERE email='a@b.com'"
                .to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get::<i64>("", "sign_in_count").unwrap()
}

#[tokio::test]
async fn records_stats_when_module_enabled() {
    let db = TestDb::new().await.unwrap();
    setup(&db).await;
    let config = AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable, AuthModule::Trackable],
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), config).await.unwrap();

    trackable::record_sign_in(db.conn(), "a@b.com", Some("1.2.3.4"))
        .await
        .unwrap();
    trackable::record_sign_in(db.conn(), "a@b.com", Some("5.6.7.8"))
        .await
        .unwrap();

    assert_eq!(sign_in_count(&db).await, 2);

    let row = db
        .conn()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT current_sign_in_at, last_sign_in_at, current_sign_in_ip, last_sign_in_ip \
             FROM users WHERE email='a@b.com'"
                .to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(row
        .try_get::<Option<String>>("", "current_sign_in_at")
        .unwrap()
        .is_some());
    assert!(row
        .try_get::<Option<String>>("", "last_sign_in_at")
        .unwrap()
        .is_some());
    // current rolls to last on the second sign-in.
    assert_eq!(
        row.try_get::<Option<String>>("", "current_sign_in_ip")
            .unwrap()
            .as_deref(),
        Some("5.6.7.8")
    );
    assert_eq!(
        row.try_get::<Option<String>>("", "last_sign_in_ip")
            .unwrap()
            .as_deref(),
        Some("1.2.3.4")
    );
}

#[tokio::test]
async fn noop_when_module_disabled() {
    let db = TestDb::new().await.unwrap();
    setup(&db).await;
    let config = AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable],
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), config).await.unwrap();

    trackable::record_sign_in(db.conn(), "a@b.com", Some("1.2.3.4"))
        .await
        .unwrap();

    assert_eq!(sign_in_count(&db).await, 0);
}
