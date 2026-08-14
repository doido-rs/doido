//! `recoverable` module — token-based password reset + reset email.

use doido_auth::config::AuthConfig;
use doido_auth::testing::init_test_auth;
use doido_auth::{recoverable, AuthModule};
use doido_mailer::TestDeliverer;
use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use doido_model::testing::TestDb;
use std::sync::Arc;

const CREATE_USERS: &str = "CREATE TABLE users (\
    id INTEGER PRIMARY KEY, email TEXT NOT NULL, password_digest TEXT, \
    reset_password_token TEXT, reset_password_sent_at TEXT)";

async fn setup(db: &TestDb, token: Option<&str>, sent_at: Option<&str>) {
    db.conn()
        .execute_raw(Statement::from_string(DbBackend::Sqlite, CREATE_USERS.to_string()))
        .await
        .unwrap();
    db.conn()
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO users (email, password_digest, reset_password_token, reset_password_sent_at) \
             VALUES ('a@b.com', 'ORIGINAL', ?, ?)",
            [
                Value::from(token.map(|s| s.to_string())),
                Value::from(sent_at.map(|s| s.to_string())),
            ],
        ))
        .await
        .unwrap();
}

fn recoverable_config() -> AuthConfig {
    AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable, AuthModule::Recoverable],
        ..Default::default()
    }
}

async fn column(db: &TestDb, col: &str) -> Option<String> {
    let row = db
        .conn()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("SELECT {col} FROM users WHERE email='a@b.com'"),
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get::<Option<String>>("", col).unwrap()
}

#[tokio::test]
async fn requests_then_resets_password() {
    let db = TestDb::new().await.unwrap();
    setup(&db, None, None).await;
    let _guard = init_test_auth(db.conn().clone(), recoverable_config()).await.unwrap();

    let token = recoverable::request_reset(db.conn(), "a@b.com")
        .await
        .unwrap()
        .expect("token for known email");
    assert_eq!(column(&db, "reset_password_token").await.as_deref(), Some(token.as_str()));

    let ok = recoverable::reset_password(db.conn(), &token, "brand-new-pass")
        .await
        .unwrap();
    assert!(ok);
    // Digest changed and the token was consumed.
    assert_ne!(column(&db, "password_digest").await.as_deref(), Some("ORIGINAL"));
    assert!(column(&db, "reset_password_token").await.is_none());
}

#[tokio::test]
async fn unknown_email_returns_no_token() {
    let db = TestDb::new().await.unwrap();
    setup(&db, None, None).await;
    let _guard = init_test_auth(db.conn().clone(), recoverable_config()).await.unwrap();

    assert!(recoverable::request_reset(db.conn(), "nobody@x.com")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn rejects_expired_or_unknown_token() {
    let db = TestDb::new().await.unwrap();
    let long_ago = (chrono::Utc::now() - chrono::Duration::seconds(100_000)).to_rfc3339();
    setup(&db, Some("expired-token"), Some(&long_ago)).await;
    let _guard = init_test_auth(db.conn().clone(), recoverable_config()).await.unwrap();

    // Expired (older than reset_password_within default).
    assert!(!recoverable::reset_password(db.conn(), "expired-token", "x").await.unwrap());
    // Unknown token.
    assert!(!recoverable::reset_password(db.conn(), "does-not-exist", "x").await.unwrap());
}

#[tokio::test]
async fn sends_reset_email_via_deliverer() {
    let db = TestDb::new().await.unwrap();
    setup(&db, None, None).await;
    let _guard = init_test_auth(db.conn().clone(), recoverable_config()).await.unwrap();

    let deliverer = TestDeliverer::new();
    // Only this test installs a deliverer, so `set_deliverer` succeeds.
    let _ = doido_mailer::global::set_deliverer(Arc::new(deliverer.clone()));

    recoverable::send_reset_email("recover-me@x.com", "tok-abc-123")
        .await
        .unwrap();

    let sent = deliverer.sent().await;
    let mail = sent
        .iter()
        .find(|m| m.to.iter().any(|t| t == "recover-me@x.com"))
        .expect("a reset email to the user");
    assert!(mail.subject.to_lowercase().contains("reset"));
    assert!(mail.body_text.as_deref().unwrap_or("").contains("tok-abc-123"));
}

#[tokio::test]
async fn noop_when_module_disabled() {
    let db = TestDb::new().await.unwrap();
    setup(&db, None, None).await;
    let cfg = AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable],
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), cfg).await.unwrap();

    assert!(recoverable::request_reset(db.conn(), "a@b.com").await.unwrap().is_none());
    assert!(!recoverable::reset_password(db.conn(), "whatever", "x").await.unwrap());
}
