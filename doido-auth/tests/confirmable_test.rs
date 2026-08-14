//! `confirmable` module — email confirmation gating.

use doido_auth::config::AuthConfig;
use doido_auth::testing::init_test_auth;
use doido_auth::{confirmable, AuthModule};
use doido_mailer::TestDeliverer;
use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use doido_model::testing::TestDb;
use std::sync::Arc;

const CREATE_USERS: &str = "CREATE TABLE users (\
    id INTEGER PRIMARY KEY, email TEXT NOT NULL, password_digest TEXT, \
    confirmation_token TEXT, confirmed_at TEXT, confirmation_sent_at TEXT)";

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

fn confirmable_config() -> AuthConfig {
    AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable, AuthModule::Confirmable],
        ..Default::default()
    }
}

async fn token(db: &TestDb) -> Option<String> {
    let row = db
        .conn()
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT confirmation_token FROM users WHERE email='a@b.com'".to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get::<Option<String>>("", "confirmation_token")
        .unwrap()
}

#[tokio::test]
async fn generate_then_confirm_flow() {
    let db = TestDb::new().await.unwrap();
    setup(&db).await;
    let _guard = init_test_auth(db.conn().clone(), confirmable_config())
        .await
        .unwrap();

    assert!(!confirmable::is_confirmed(db.conn(), "a@b.com")
        .await
        .unwrap());

    let tok = confirmable::generate_confirmation(db.conn(), "a@b.com")
        .await
        .unwrap()
        .expect("token");
    assert_eq!(token(&db).await.as_deref(), Some(tok.as_str()));

    assert!(confirmable::confirm(db.conn(), &tok).await.unwrap());
    assert!(confirmable::is_confirmed(db.conn(), "a@b.com")
        .await
        .unwrap());
    assert!(token(&db).await.is_none(), "token cleared after confirm");

    // Unknown token is rejected.
    assert!(!confirmable::confirm(db.conn(), "nope").await.unwrap());
}

#[tokio::test]
async fn sends_confirmation_email() {
    let db = TestDb::new().await.unwrap();
    setup(&db).await;
    let _guard = init_test_auth(db.conn().clone(), confirmable_config())
        .await
        .unwrap();

    let deliverer = TestDeliverer::new();
    let _ = doido_mailer::global::set_deliverer(Arc::new(deliverer.clone()));

    confirmable::send_confirmation_email("a@b.com", "conf-tok-1")
        .await
        .unwrap();

    let sent = deliverer.sent().await;
    let mail = sent
        .iter()
        .find(|m| m.to.iter().any(|t| t == "a@b.com"))
        .expect("a confirmation email");
    assert!(mail.subject.to_lowercase().contains("confirm"));
    assert!(mail
        .body_text
        .as_deref()
        .unwrap_or("")
        .contains("conf-tok-1"));
}

#[tokio::test]
async fn disabled_treats_accounts_as_confirmed() {
    let db = TestDb::new().await.unwrap();
    setup(&db).await;
    let cfg = AuthConfig {
        modules: vec![AuthModule::DatabaseAuthenticatable],
        ..Default::default()
    };
    let _guard = init_test_auth(db.conn().clone(), cfg).await.unwrap();

    // No gating when disabled.
    assert!(confirmable::is_confirmed(db.conn(), "a@b.com")
        .await
        .unwrap());
    assert!(confirmable::generate_confirmation(db.conn(), "a@b.com")
        .await
        .unwrap()
        .is_none());
    assert!(!confirmable::confirm(db.conn(), "x").await.unwrap());
}
