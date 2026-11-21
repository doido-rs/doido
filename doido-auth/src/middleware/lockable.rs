//! Account lock after failed sign-ins.

use super::context::SignInContext;
use super::trait_::AuthMiddleware;
use crate::config::AuthModule;
use crate::error::AuthError;
use crate::state::try_global;
use crate::user_model::plan::{FetchContribution, UserField};
use crate::user_model::row::UserRow;
use crate::user_model::{UserPatchBuilder, UserStore};
use async_trait::async_trait;
use chrono::Utc;

pub struct LockableMiddleware;

fn settings() -> Option<(u32, i64)> {
    let state = try_global()?;
    if !state.config.has_module(AuthModule::Lockable) {
        return None;
    }
    Some((state.config.maximum_attempts, state.config.unlock_in))
}

/// Lock check from a row already loaded; auto-unlocks when the window elapsed.
pub async fn prepare_from_row(
    store: &dyn UserStore,
    email: &str,
    row: &UserRow,
) -> Result<(), AuthError> {
    let (_, unlock_in) = match settings() {
        Some(s) => s,
        None => return Ok(()),
    };
    let locked = match row.locked_at.as_deref() {
        Some(ts) => ts,
        None => return Ok(()),
    };
    let locked_time = chrono::DateTime::parse_from_rfc3339(locked)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    if (Utc::now() - locked_time).num_seconds() >= unlock_in {
        unlock(store, email).await
    } else {
        Err(AuthError::AccountLocked)
    }
}

/// Validate lock state from a row already loaded (no extra query).
pub fn validate_row(row: &UserRow) -> Result<(), AuthError> {
    let (_, unlock_in) = match settings() {
        Some(s) => s,
        None => return Ok(()),
    };
    let locked = match row.locked_at.as_deref() {
        Some(ts) => ts,
        None => return Ok(()),
    };
    let locked_time = chrono::DateTime::parse_from_rfc3339(locked)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    if (Utc::now() - locked_time).num_seconds() >= unlock_in {
        Ok(())
    } else {
        Err(AuthError::AccountLocked)
    }
}

async fn unlock(store: &dyn UserStore, email: &str) -> Result<(), AuthError> {
    let action = UserPatchBuilder::by_email(email)
        .set(UserField::FailedAttempts, "0")
        .clear(UserField::LockedAt)
        .build()?;
    store.apply(action).await
}

pub async fn record_failure(store: &dyn UserStore, email: &str) -> Result<(), AuthError> {
    let (maximum, _) = match settings() {
        Some(s) => s,
        None => return Ok(()),
    };
    let action = UserPatchBuilder::by_email(email)
        .increment(UserField::FailedAttempts)
        .build()?;
    store.apply(action).await?;

    let row = store
        .fetch_by_email(
            email,
            &crate::user_model::FetchPlan::minimal().merge(
                FetchContribution::default()
                    .with(UserField::FailedAttempts)
                    .with(UserField::LockedAt),
            ),
        )
        .await?;
    let attempts = row.and_then(|r| r.failed_attempts).unwrap_or(0);
    if attempts as u32 >= maximum {
        let action = UserPatchBuilder::by_email(email)
            .set(UserField::LockedAt, Utc::now().to_rfc3339())
            .build()?;
        store.apply(action).await?;
    }
    Ok(())
}

#[async_trait]
impl AuthMiddleware for LockableMiddleware {
    fn name(&self) -> &'static str {
        "lockable"
    }

    fn sign_in_fetch_plan(&self) -> crate::user_model::FetchContribution {
        FetchContribution::default()
            .with(UserField::FailedAttempts)
            .with(UserField::LockedAt)
    }

    async fn after_sign_in_failed(&self, ctx: &SignInContext<'_>) -> Result<(), AuthError> {
        let _ = record_failure(ctx.store, ctx.email).await;
        Ok(())
    }

    async fn after_sign_in_success(&self, ctx: &SignInContext<'_>) -> Result<(), AuthError> {
        if settings().is_some() {
            let _ = unlock(ctx.store, ctx.email).await;
        }
        Ok(())
    }
}

/// Clear lock state after a successful sign-in (legacy name).
pub async fn reset_attempts(store: &dyn UserStore, email: &str) -> Result<(), AuthError> {
    if settings().is_none() {
        return Ok(());
    }
    unlock(store, email).await
}

pub async fn ensure_not_locked(store: &dyn UserStore, email: &str) -> Result<(), AuthError> {
    let plan = crate::user_model::FetchPlan::minimal().merge(
        FetchContribution::default()
            .with(UserField::FailedAttempts)
            .with(UserField::LockedAt),
    );
    let row = store.fetch_by_email(email, &plan).await?;
    match row {
        Some(r) => prepare_from_row(store, email, &r).await,
        None => Ok(()),
    }
}
