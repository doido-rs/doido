//! Absolute session age expiry.

use super::trait_::AuthMiddleware;
use crate::config::AuthModule;
use crate::state::try_global;
use async_trait::async_trait;
use doido_controller::session::Session;

pub struct TimeoutableMiddleware;

/// Whether the session has exceeded `auth.timeout` since sign-in.
pub fn is_session_expired(session: &Session) -> bool {
    let state = match try_global() {
        Some(state) => state,
        None => return false,
    };
    if !state.config.has_module(AuthModule::Timeoutable) {
        return false;
    }
    let signed_in_at = match session.data.get(crate::strategy::session::SIGNED_IN_AT_KEY) {
        Some(v) => v.as_i64(),
        None => return false,
    };
    let Some(signed_in_at) = signed_in_at else {
        return false;
    };
    let elapsed = chrono::Utc::now().timestamp() - signed_in_at;
    elapsed > state.config.timeout as i64
}

#[async_trait]
impl AuthMiddleware for TimeoutableMiddleware {
    fn name(&self) -> &'static str {
        "timeoutable"
    }
}
