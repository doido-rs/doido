//! Process-global auth state installed at boot.

use crate::config::AuthConfig;
use crate::error::AuthError;
use crate::jwt::JwtStrategy;
use crate::oauth::{providers_from_config, OAuthProvider};
use crate::registry::{all_custom_strategies, get_strategy};
use crate::strategy::AuthStrategy;
use doido_core::Result;
use doido_model::sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Boot-time auth state: DB handle, config, and enabled strategies.
pub struct AuthState {
    pub db: DatabaseConnection,
    pub config: AuthConfig,
    pub strategies: Vec<Arc<dyn AuthStrategy>>,
    pub jwt: Option<Arc<JwtStrategy>>,
    pub oauth: HashMap<String, Arc<dyn OAuthProvider>>,
}

static AUTH_STATE: OnceLock<RwLock<Option<Arc<AuthState>>>> = OnceLock::new();

fn slot() -> &'static RwLock<Option<Arc<AuthState>>> {
    AUTH_STATE.get_or_init(|| RwLock::new(None))
}

impl AuthState {
    pub(crate) fn build(db: DatabaseConnection, config: AuthConfig) -> Result<Self, AuthError> {
        config.validate()?;
        let mut strategies: Vec<Arc<dyn AuthStrategy>> = Vec::new();
        let mut jwt = None;

        for name in &config.strategies {
            match name.as_str() {
                "cookie" => {
                    strategies.push(Arc::new(crate::session::from_config(&config)));
                }
                "jwt" => {
                    let jwt_cfg = config
                        .jwt
                        .clone()
                        .ok_or_else(|| AuthError::Config("missing auth.jwt".into()))?;
                    let strategy = Arc::new(JwtStrategy::new(jwt_cfg)?);
                    jwt = Some(strategy.clone());
                    strategies.push(strategy);
                }
                custom => {
                    let strategy = get_strategy(custom)
                        .ok_or_else(|| AuthError::UnknownStrategy(custom.to_string()))?;
                    strategies.push(strategy);
                }
            }
        }

        for custom in all_custom_strategies() {
            if !strategies.iter().any(|s| s.name() == custom.name())
                && config.strategies.iter().any(|n| n == custom.name())
            {
                strategies.push(custom);
            }
        }

        let oauth = providers_from_config(&config.oauth);

        Ok(Self {
            db,
            config,
            strategies,
            jwt,
            oauth,
        })
    }
}

/// Initialise auth at boot. Idempotent: returns Ok when already initialised.
pub async fn init(db: DatabaseConnection, config: &AuthConfig) -> Result<()> {
    let mut guard = slot().write().expect("auth state lock");
    if guard.is_some() {
        return Ok(());
    }
    let state = AuthState::build(db, config.clone()).map_err(|e| doido_core::anyhow::anyhow!(e))?;
    *guard = Some(Arc::new(state));
    Ok(())
}

/// Install or replace auth state (tests and advanced boot scenarios).
pub fn set_state(state: AuthState) {
    *slot().write().expect("auth state lock") = Some(Arc::new(state));
}

/// Returns the global auth state, panicking if [`init`] was never called.
pub fn global() -> Arc<AuthState> {
    try_global().expect("auth not initialised; call doido_auth::init() at boot")
}

/// Returns the global auth state when installed.
pub fn try_global() -> Option<Arc<AuthState>> {
    slot().read().expect("auth state lock").clone()
}

/// Clear auth state (tests only).
pub fn reset_state() {
    *slot().write().expect("auth state lock") = None;
}
