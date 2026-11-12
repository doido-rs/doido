//! Built-in and custom auth middleware registration.

use super::trait_::AuthMiddleware;
use crate::config::{AuthConfig, AuthModule};
use std::sync::{Arc, OnceLock, RwLock};

fn custom_slot() -> &'static RwLock<Vec<Arc<dyn AuthMiddleware>>> {
    static CUSTOM: OnceLock<RwLock<Vec<Arc<dyn AuthMiddleware>>>> = OnceLock::new();
    CUSTOM.get_or_init(|| RwLock::new(Vec::new()))
}

/// Register an app-defined auth middleware (runs after built-ins).
pub fn register_middleware(mw: Arc<dyn AuthMiddleware>) {
    custom_slot()
        .write()
        .expect("auth middleware lock")
        .push(mw);
}

/// Clear custom middleware (tests).
pub fn reset_custom_middleware() {
    *custom_slot().write().expect("auth middleware lock") = Vec::new();
}

pub fn all_middlewares(config: &AuthConfig) -> Vec<Arc<dyn AuthMiddleware>> {
    let mut list = built_in_middlewares(config);
    list.extend(custom_slot().read().expect("auth middleware lock").clone());
    list
}

pub fn built_in_middlewares(config: &AuthConfig) -> Vec<Arc<dyn AuthMiddleware>> {
    let mut out: Vec<Arc<dyn AuthMiddleware>> = Vec::new();
    if config.has_module(AuthModule::Validatable) {
        out.push(Arc::new(super::validatable::ValidatableMiddleware));
    }
    if config.has_module(AuthModule::Lockable) {
        out.push(Arc::new(super::lockable::LockableMiddleware));
    }
    if config.has_module(AuthModule::Confirmable) {
        out.push(Arc::new(super::confirmable::ConfirmableMiddleware));
    }
    if config.has_module(AuthModule::Trackable) {
        out.push(Arc::new(super::trackable::TrackableMiddleware));
    }
    if config.has_module(AuthModule::Rememberable) {
        out.push(Arc::new(super::rememberable::RememberableMiddleware));
    }
    if config.has_module(AuthModule::Recoverable) {
        out.push(Arc::new(super::recoverable::RecoverableMiddleware));
    }
    if config.has_module(AuthModule::Timeoutable) {
        out.push(Arc::new(super::timeoutable::TimeoutableMiddleware));
    }
    #[cfg(feature = "auth-2fa")]
    if config.has_module(AuthModule::TwoFactorAuthenticatable) {
        out.push(Arc::new(super::two_factorable::TwoFactorableMiddleware));
    }
    out
}
