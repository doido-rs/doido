//! Sign-in statistics.

use super::context::SignInContext;
use super::trait_::AuthMiddleware;
use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use crate::user_model::{UserStore, UserStoreAction};
use async_trait::async_trait;
use chrono::Utc;

pub struct TrackableMiddleware;

fn enabled() -> bool {
    matches!(try_global(), Some(state) if state.config.has_module(AuthModule::Trackable))
}

pub async fn record_sign_in(
    store: &dyn UserStore,
    email: &str,
    ip: Option<&str>,
) -> Result<(), AuthError> {
    if !enabled() {
        return Ok(());
    }
    store
        .apply(UserStoreAction::RecordSuccessfulSignIn {
            email: email.to_string(),
            signed_in_at: Utc::now().to_rfc3339(),
            ip: ip.map(|s| s.to_string()),
        })
        .await
}

#[async_trait]
impl AuthMiddleware for TrackableMiddleware {
    fn name(&self) -> &'static str {
        "trackable"
    }

    async fn after_sign_in_success(&self, ctx: &SignInContext<'_>) -> Result<(), AuthError> {
        let _ = record_sign_in(ctx.store, ctx.email, None).await;
        Ok(())
    }
}
