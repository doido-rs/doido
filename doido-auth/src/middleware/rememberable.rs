//! Persistent "remember me" cookie (DB side).

use super::context::SignOutContext;
use super::trait_::AuthMiddleware;
use crate::config::AuthModule;
use crate::error::AuthError;
use crate::user_model::plan::UserField;
use crate::user_model::{UserPatchBuilder, UserStore};
use async_trait::async_trait;
use chrono::Utc;

pub struct RememberableMiddleware;

pub const REMEMBER_COOKIE: &str = "_doido_remember";

fn enabled() -> bool {
    matches!(
        crate::state::try_global(),
        Some(state) if state.config.has_module(AuthModule::Rememberable)
    )
}

pub fn cookie_value(user_id: &impl serde::Serialize) -> String {
    serde_json::to_string(user_id).unwrap_or_default()
}

pub async fn record_remember(store: &dyn UserStore, email: &str) -> Result<(), AuthError> {
    if !enabled() {
        return Ok(());
    }
    let action = UserPatchBuilder::by_email(email)
        .set(UserField::RememberCreatedAt, Utc::now().to_rfc3339())
        .build()?;
    store.apply(action).await
}

pub async fn forget(store: &dyn UserStore, email: &str) -> Result<(), AuthError> {
    if !enabled() {
        return Ok(());
    }
    let action = UserPatchBuilder::by_email(email)
        .clear(UserField::RememberCreatedAt)
        .build()?;
    store.apply(action).await
}

#[async_trait]
impl AuthMiddleware for RememberableMiddleware {
    fn name(&self) -> &'static str {
        "rememberable"
    }

    async fn before_sign_out(&self, ctx: &mut SignOutContext<'_>) -> Result<(), AuthError> {
        if let Some(email) = ctx.email.as_deref() {
            let _ = forget(ctx.store, email).await;
        }
        Ok(())
    }
}
