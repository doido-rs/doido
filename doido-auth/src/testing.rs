//! Test helpers and in-memory fakes for auth integration tests.

use crate::config::AuthConfig;
use crate::error::AuthError;
use crate::handlers::{register_user, sign_in_with_session};
use crate::state::{set_state, AuthState};
use crate::strategy::jwt::JwtStrategy;
use crate::user::{RegisterableUserModel, UserModel};
use crate::user_model::plan::FetchPlan;
use crate::user_model::row::UserRow;
use crate::user_model::UserStore;
use doido_controller::axum::body::Body;
use doido_controller::axum::Router;
use doido_controller::session::Session;
use doido_model::password::{hash_password_with_cost, HasSecurePassword};
use doido_model::sea_orm::DatabaseConnection;
use http::{Request, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const TEST_COST: u32 = 4;

static AUTH_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct AuthTestGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestUser {
    pub id: i64,
    pub email: String,
    pub password_digest: String,
}

impl HasSecurePassword for TestUser {
    fn password_digest(&self) -> &str {
        &self.password_digest
    }
}

impl UserModel for TestUser {
    type Id = i64;

    fn from_row(row: UserRow) -> Result<Self, AuthError> {
        Ok(Self {
            id: row.id.and_then(|v| v.as_i64()).unwrap_or_default(),
            email: row.email.unwrap_or_default(),
            password_digest: row.password_digest.unwrap_or_default(),
        })
    }

    fn id(&self) -> Self::Id {
        self.id
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn password_digest(&self) -> Option<&str> {
        Some(&self.password_digest)
    }

    async fn find_by_email(
        store: &dyn UserStore,
        email: &str,
        plan: &FetchPlan,
    ) -> doido_core::Result<Option<Self>> {
        crate::user_model::find_by_email_mapped::<Self>(store, email, plan)
            .await
            .map_err(Into::into)
    }

    async fn find_by_id(
        store: &dyn UserStore,
        id: Self::Id,
        plan: &FetchPlan,
    ) -> doido_core::Result<Option<Self>> {
        crate::user_model::find_by_id_mapped::<Self>(store, id, plan)
            .await
            .map_err(Into::into)
    }

    async fn insert(_store: &dyn UserStore, row: UserRow) -> doido_core::Result<Self> {
        Self::from_row(row).map_err(Into::into)
    }
}

impl RegisterableUserModel for TestUser {
    async fn register(
        store: &dyn UserStore,
        email: String,
        password_digest: String,
    ) -> doido_core::Result<Self> {
        if let Some(db) = store.sea_orm() {
            use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
            db.execute_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO users (email, password_digest) VALUES (?, ?)",
                [
                    Value::from(email.clone()),
                    Value::from(password_digest.clone()),
                ],
            ))
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT id FROM users WHERE email = ?",
                    [Value::from(email.clone())],
                ))
                .await
                .map_err(|e| AuthError::Internal(e.to_string()))?;
            let id = row
                .and_then(|r| r.try_get::<i64>("", "id").ok())
                .unwrap_or(1);
            let user = TestUser {
                id,
                email,
                password_digest,
            };
            test_users_map().lock().unwrap().insert(id, user.clone());
            return Ok(user);
        }
        let users = test_users_map();
        let mut map = users.lock().unwrap();
        let id = (map.len() as i64) + 1;
        let user = TestUser {
            id,
            email,
            password_digest,
        };
        map.insert(id, user.clone());
        Ok(user)
    }
}

static TEST_STORE: std::sync::OnceLock<Arc<Mutex<HashMap<i64, TestUser>>>> =
    std::sync::OnceLock::new();

fn test_users_map() -> Arc<Mutex<HashMap<i64, TestUser>>> {
    Arc::clone(TEST_STORE.get_or_init(|| Arc::new(Mutex::new(HashMap::new()))))
}

pub fn reset_store() {
    test_users_map().lock().unwrap().clear();
}

/// The process-global [`UserStore`] installed by [`init_test_auth`].
pub fn auth_store() -> std::sync::Arc<dyn crate::user_model::UserStore> {
    crate::state::global().store.clone()
}

pub fn test_auth_config() -> AuthConfig {
    AuthConfig {
        modules: vec![crate::config::AuthModule::DatabaseAuthenticatable],
        ..Default::default()
    }
}

pub fn test_jwt_auth_config(secret: &str) -> AuthConfig {
    AuthConfig {
        strategies: vec!["cookie".into(), "jwt".into()],
        jwt: Some(crate::config::JwtConfig {
            secret: secret.into(),
            access_ttl: 900,
            refresh_ttl: 604_800,
            issuer: Some("test".into()),
        }),
        ..Default::default()
    }
}

pub async fn init_test_auth(
    db: DatabaseConnection,
    config: AuthConfig,
) -> Result<AuthTestGuard, AuthError> {
    let guard = AUTH_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    reset_store();
    crate::state::reset_state();
    let _ = doido_model::pool::set_pool(db.clone());
    set_state(AuthState::build(db, config)?);
    Ok(AuthTestGuard { _lock: guard })
}

pub async fn ensure_users_table(db: &DatabaseConnection) -> Result<(), AuthError> {
    use doido_model::sea_orm::{ConnectionTrait, DbBackend, Statement};
    db.execute_raw(Statement::from_string::<String>(
        DbBackend::Sqlite,
        "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE, password_digest TEXT NOT NULL)".into(),
    ))
    .await
    .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(())
}

pub async fn create_test_user(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
) -> Result<TestUser, AuthError> {
    ensure_users_table(db).await?;
    use crate::state::global;
    let store = global().store.clone();
    register_user::<TestUser, _, _>(email, password, |email, digest| async move {
        TestUser::register(store.as_ref(), email, digest).await
    })
    .await
}

pub fn jwt_for_user(config: &crate::config::JwtConfig, user_id: i64) -> String {
    let strategy = JwtStrategy::new(config.clone()).expect("jwt config");
    strategy
        .issue_tokens(&serde_json::json!(user_id))
        .expect("issue tokens")
        .access_token
}

pub struct TestResponse {
    pub status: StatusCode,
    pub body: String,
    pub set_cookie: Option<String>,
}

pub async fn send(router: Router, method: &str, uri: &str, body: &str) -> TestResponse {
    send_with_headers(router, method, uri, body, &[]).await
}

pub async fn send_with_headers(
    router: Router,
    method: &str,
    uri: &str,
    body: &str,
    headers: &[(&str, &str)],
) -> TestResponse {
    let mut builder = Request::builder().method(method).uri(uri);
    if !body.is_empty() && (method == "POST" || method == "PATCH") {
        builder = builder
            .header(http::header::CONTENT_TYPE, "application/json")
            .header(http::header::ACCEPT, "application/json");
    }
    for (k, v) in headers {
        builder = builder.header(*k, *v);
    }
    let request = builder
        .body(Body::from(body.to_string()))
        .expect("valid test request");
    let response = router
        .oneshot(request)
        .await
        .expect("router handled the request");
    let status = response.status();
    let set_cookie = response
        .headers()
        .get(http::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = doido_controller::axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    TestResponse {
        status,
        body: String::from_utf8_lossy(&bytes).to_string(),
        set_cookie,
    }
}

pub fn hash_test_password(password: &str) -> String {
    hash_password_with_cost(password, TEST_COST).expect("hash")
}

pub fn session_for_user(user: &TestUser) -> Session {
    let mut session = Session::new();
    sign_in_with_session(&mut session, user);
    session
}

pub use crate::strategy::jwt::JwtStrategy as TestJwtStrategy;
pub use crate::strategy::session::SessionStrategy as TestSessionStrategy;
