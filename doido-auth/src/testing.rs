//! Test helpers and in-memory fakes for auth integration tests.

use crate::config::AuthConfig;
use crate::error::AuthError;
use crate::handlers::{register_user, sign_in_with_session};
use crate::jwt::JwtStrategy;
use crate::state::{set_state, AuthState};
use crate::user::AuthUser;
use crate::RegisterableAuthUser;
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

/// Holds the auth test lock for the duration of a test (serialises global auth state).
pub struct AuthTestGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

/// Simple in-memory user for auth tests.
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

impl AuthUser for TestUser {
    type Id = i64;

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
        _db: &DatabaseConnection,
        email: &str,
    ) -> doido_core::Result<Option<Self>> {
        Ok(store()
            .lock()
            .unwrap()
            .values()
            .find(|u| u.email == email)
            .cloned())
    }

    async fn find_by_id(
        _db: &DatabaseConnection,
        id: Self::Id,
    ) -> doido_core::Result<Option<Self>> {
        Ok(store().lock().unwrap().get(&id).cloned())
    }
}

impl RegisterableAuthUser for TestUser {
    async fn register(
        _db: &DatabaseConnection,
        email: String,
        password_digest: String,
    ) -> doido_core::Result<Self> {
        let store = store();
        let mut map = store.lock().unwrap();
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

fn store() -> Arc<Mutex<HashMap<i64, TestUser>>> {
    TEST_STORE
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

/// Reset the in-memory user store between tests.
pub fn reset_store() {
    store().lock().unwrap().clear();
}

/// Build a minimal auth config for tests: cookie strategy, only the
/// `database_authenticatable` module. Kept minimal so module behaviors
/// (validatable, trackable, …) don't implicitly affect unrelated tests — a test
/// that exercises a module should enable it explicitly.
pub fn test_auth_config() -> AuthConfig {
    AuthConfig {
        modules: vec![crate::config::AuthModule::DatabaseAuthenticatable],
        ..Default::default()
    }
}

/// Build auth config with JWT enabled.
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

/// Initialise in-memory auth state for tests.
///
/// Returns a guard that must be held for the whole test so parallel tests do not
/// clobber process-global auth state.
pub async fn init_test_auth(
    db: DatabaseConnection,
    config: AuthConfig,
) -> Result<AuthTestGuard, AuthError> {
    let guard = AUTH_TEST_LOCK.lock().expect("auth test lock");
    reset_store();
    crate::state::reset_state();
    let _ = doido_model::pool::set_pool(db.clone());
    set_state(AuthState::build(db, config)?);
    Ok(AuthTestGuard { _lock: guard })
}

/// Create a user in the in-memory store.
pub async fn create_test_user(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
) -> Result<TestUser, AuthError> {
    let users = store();
    register_user(db, email, password, |email, digest| async move {
        let mut map = users.lock().unwrap();
        let id = (map.len() as i64) + 1;
        let user = TestUser {
            id,
            email,
            password_digest: digest,
        };
        map.insert(id, user.clone());
        Ok(user)
    })
    .await
}

/// Issue a signed JWT access token for a test user id.
pub fn jwt_for_user(config: &crate::config::JwtConfig, user_id: i64) -> String {
    let strategy = JwtStrategy::new(config.clone()).expect("jwt config");
    strategy
        .issue_tokens(&serde_json::json!(user_id))
        .expect("issue tokens")
        .access_token
}

/// The status + body captured from a test request.
pub struct TestResponse {
    pub status: StatusCode,
    pub body: String,
    pub set_cookie: Option<String>,
}

/// Send a request to `router` in-process and capture the response.
pub async fn send(router: Router, method: &str, uri: &str, body: &str) -> TestResponse {
    send_with_headers(router, method, uri, body, &[]).await
}

/// Send a request with extra headers.
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

/// Hash a password at the low test cost.
pub fn hash_test_password(password: &str) -> String {
    hash_password_with_cost(password, TEST_COST).expect("hash")
}

/// Sign a user into a fresh session (helper for session strategy tests).
pub fn session_for_user(user: &TestUser) -> Session {
    let mut session = Session::new();
    sign_in_with_session(&mut session, user);
    session
}

pub use crate::jwt::JwtStrategy as TestJwtStrategy;
pub use crate::session::SessionStrategy as TestSessionStrategy;
