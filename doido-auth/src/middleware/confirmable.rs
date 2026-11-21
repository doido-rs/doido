//! Email confirmation before sign-in.

use super::trait_::AuthMiddleware;
use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use crate::user_model::plan::{FetchContribution, UserField};
use crate::user_model::row::UserRow;
use crate::user_model::{UserPatchBuilder, UserStore};
use async_trait::async_trait;
use doido_mailer::Mail;

pub struct ConfirmableMiddleware;

pub fn is_enabled() -> bool {
    matches!(try_global(), Some(state) if state.config.has_module(AuthModule::Confirmable))
}

pub fn validate_row(row: &UserRow) -> Result<(), AuthError> {
    if !is_enabled() {
        return Ok(());
    }
    if row.confirmed_at.is_some() {
        Ok(())
    } else {
        Err(AuthError::NotConfirmed)
    }
}

pub async fn is_confirmed(store: &dyn UserStore, email: &str) -> Result<bool, AuthError> {
    if !is_enabled() {
        return Ok(true);
    }
    let plan = crate::user_model::FetchPlan::minimal()
        .merge(FetchContribution::default().with(UserField::ConfirmedAt));
    let row = store.fetch_by_email(email, &plan).await?;
    Ok(row.is_some_and(|r| r.confirmed_at.is_some()))
}

pub async fn generate_confirmation(
    store: &dyn UserStore,
    email: &str,
) -> Result<Option<String>, AuthError> {
    if !is_enabled() {
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
        .set(UserField::ConfirmationToken, &token)
        .set(UserField::ConfirmationSentAt, &sent_at)
        .index_token(UserField::ConfirmationToken, &token)
        .build()?;
    store.apply(action).await?;
    Ok(Some(token))
}

pub async fn confirm(store: &dyn UserStore, token: &str) -> Result<bool, AuthError> {
    if !is_enabled() {
        return Ok(false);
    }
    let exists = store
        .fetch_by_field(
            UserField::ConfirmationToken,
            token,
            &crate::user_model::FetchPlan::minimal(),
        )
        .await?
        .is_some();
    if !exists {
        return Ok(false);
    }
    let confirmed_at = chrono::Utc::now().to_rfc3339();
    let action = UserPatchBuilder::by_field(UserField::ConfirmationToken, token)
        .set(UserField::ConfirmedAt, &confirmed_at)
        .clear(UserField::ConfirmationToken)
        .clear(UserField::ConfirmationSentAt)
        .drop_token_index(UserField::ConfirmationToken, token)
        .build()?;
    store.apply(action).await?;
    Ok(true)
}

pub async fn send_confirmation_email(email: &str, token: &str) -> Result<(), AuthError> {
    let prefix = match try_global() {
        Some(state) => state.config.routes.prefix.trim_end_matches('/').to_string(),
        None => "/users".to_string(),
    };
    let url = format!("{prefix}/confirmation?confirmation_token={token}");
    let mail = Mail::new()
        .to(email)
        .subject("Confirm your email")
        .body_text(format!(
            "Welcome! Confirm your email address to activate your account:\n\n{url}\n"
        ));
    doido_mailer::global::deliverer()
        .deliver(&mail)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))
}

#[async_trait]
impl AuthMiddleware for ConfirmableMiddleware {
    fn name(&self) -> &'static str {
        "confirmable"
    }

    fn sign_in_fetch_plan(&self) -> FetchContribution {
        FetchContribution::default().with(UserField::ConfirmedAt)
    }
}
