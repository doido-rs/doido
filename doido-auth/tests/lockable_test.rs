//! `lockable` module — lock after repeated failures, time-based auto-unlock.

use doido_auth::config::AuthConfig;
use doido_auth::testing::init_test_auth;
use doido_auth::{lockable, AuthError, AuthModule};
use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use doido_model::testing::TestDb;

const CREATE_USERS: &str = "CREATE TABLE users (\
    id INTEGER PRIMARY KEY, email TEXT NOT NULL, password_digest TEXT, \
    failed_attempts INTEGER NOT NULL DEFAULT 0, locked_at TEXT)";

async fn insert_user(db: &TestDb, locked_at: Option<&str>) {
    db.conn()
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            CREATE_USERS.to_string(),
        ))
        .await
        .unwrap();
    db.conn()
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO users (email, password_digest, locked_at) VALUES ('a@b.com', 'x', ?)",
            [Value::from(locked_at.map(|s| s.to_string()))],
        ))
        .await
        .unwrap();
}

fn config(maximum_attempts: u32, unlock_in: i64) -> AuthConfig {
    AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable, AuthModule::Lockable],
        maximum_attempts,
        unlock_in,
        ..Default::default()
    }
}

#[tokio::test]
async fn locks_after_maximum_attempts_then_rejects() {
    let db = TestDb::new().await.unwrap();
    insert_user(&db, None).await;
    let _guard = init_test_auth(db.conn().clone(), config(3, 3600))
        .await
        .unwrap();

    // Not locked yet.
    lockable::ensure_not_locked(db.conn(), "a@b.com")
        .await
        .unwrap();

    for _ in 0..3 {
        lockable::record_failure(db.conn(), "a@b.com")
            .await
            .unwrap();
    }

    let err = lockable::ensure_not_locked(db.conn(), "a@b.com")
        .await
        .unwrap_err();
    assert!(matches!(err, AuthError::AccountLocked));

    // A successful sign-in resets the counter and unlocks.
    lockable::reset_attempts(db.conn(), "a@b.com")
        .await
        .unwrap();
    lockable::ensure_not_locked(db.conn(), "a@b.com")
        .await
        .unwrap();
}

#[tokio::test]
async fn auto_unlocks_after_window() {
    let db = TestDb::new().await.unwrap();
    // Locked two minutes ago.
    let two_min_ago = (chrono::Utc::now() - chrono::Duration::seconds(120)).to_rfc3339();
    insert_user(&db, Some(&two_min_ago)).await;
    let _guard = init_test_auth(db.conn().clone(), config(3, 60))
        .await
        .unwrap();

    // Window (60s) has elapsed — the attempt is allowed and the lock cleared.
    lockable::ensure_not_locked(db.conn(), "a@b.com")
        .await
        .unwrap();

    let row = db
        .conn()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT locked_at FROM users WHERE email='a@b.com'".to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(row
        .try_get::<Option<String>>("", "locked_at")
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn noop_when_module_disabled() {
    let db = TestDb::new().await.unwrap();
    insert_user(&db, None).await;
    let cfg = AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable],
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), cfg).await.unwrap();

    // No columns touched, no lock enforced.
    for _ in 0..50 {
        lockable::record_failure(db.conn(), "a@b.com")
            .await
            .unwrap();
    }
    lockable::ensure_not_locked(db.conn(), "a@b.com")
        .await
        .unwrap();
}
