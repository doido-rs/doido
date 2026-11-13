//! Password reset tokens.

use super::trait_::AuthMiddleware;
use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use crate::user_model::plan::{FetchContribution, UserField};
use crate::user_model::{UserPatchBuilder, UserStore};
use async_trait::async_trait;
use doido_mailer::Mail;
use doido_model::password::hash_password;

pub struct RecoverableMiddleware;

fn enabled() -> bool {
    matches!(try_global(), Some(state) if state.config.has_module(AuthModule::Recoverable))
}

pub async fn request_reset(
    store: &dyn UserStore,
    email: &str,
) -> Result<Option<String>, AuthError> {
    if !enabled() {
        return Ok(None);
    }
    let exists = store
        .fetch_by_email(email, &crate::user_model::FetchPlan::minimal())
        .await?
        .is_some();
    if !exists {
        return Ok(None);
    }
    let token = uuid::Uuid::new_v4().to_string();
    let sent_at = chrono::Utc::now().to_rfc3339();
    let action = UserPatchBuilder::by_email(email)
        .set(UserField::ResetPasswordToken, &token)
        .set(UserField::ResetPasswordSentAt, &sent_at)
        .index_token(UserField::ResetPasswordToken, &token)
        .build()?;
    store.apply(action).await?;
    Ok(Some(token))
}

pub async fn reset_password(
    store: &dyn UserStore,
    token: &str,
    new_password: &str,
) -> Result<bool, AuthError> {
    let within = match try_global() {
        Some(state) if state.config.has_module(AuthModule::Recoverable) => {
            state.config.reset_password_within
        }
        _ => return Ok(false),
    };

    let plan = crate::user_model::FetchPlan::minimal()
        .merge(FetchContribution::default().with(UserField::ResetPasswordSentAt));
    let row = store
        .fetch_by_field(UserField::ResetPasswordToken, token, &plan)
        .await?;
    let sent_at = match row {
        Some(r) => r.reset_password_sent_at,
        None => return Ok(false),
    };
    if let Some(sent_at) = sent_at {
        let sent = chrono::DateTime::parse_from_rfc3339(&sent_at)
            .map(|t| t.with_timezone(&chrono::Utc))
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        if (chrono::Utc::now() - sent).num_seconds() > within {
            return Ok(false);
        }
    }

    let digest = hash_password(new_password).map_err(|e| AuthError::Internal(e.to_string()))?;
    let action = UserPatchBuilder::by_field(UserField::ResetPasswordToken, token)
        .set(UserField::PasswordDigest, &digest)
        .clear(UserField::ResetPasswordToken)
        .clear(UserField::ResetPasswordSentAt)
        .drop_token_index(UserField::ResetPasswordToken, token)
        .build()?;
    store.apply(action).await?;
    Ok(true)
}

pub async fn send_reset_email(email: &str, token: &str) -> Result<(), AuthError> {
    let prefix = match try_global() {
        Some(state) => state.config.routes.prefix.trim_end_matches('/').to_string(),
        None => "/users".to_string(),
    };
    let url = format!("{prefix}/password/edit?reset_password_token={token}");
    let mail = Mail::new()
        .to(email)
        .subject("Reset your password")
        .body_text(format!(
            "Someone requested a password reset. Use this link to choose a new password:\n\n{url}\n\nIf you didn't request this, ignore this email."
        ));
    doido_mailer::global::deliverer()
        .deliver(&mail)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))
}

#[async_trait]
impl AuthMiddleware for RecoverableMiddleware {
    fn name(&self) -> &'static str {
        "recoverable"
    }
}
