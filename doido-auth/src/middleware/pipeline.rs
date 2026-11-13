//! Auth lifecycle orchestration — one read for sign-in validation.

use super::confirmable;
use super::context::{SignInContext, SignOutContext, SignUpContext, ValidateContext};
use super::lockable;
use super::registry::all_middlewares;
use crate::error::AuthError;
use crate::state::try_global;
use crate::user_model::plan::FetchPlan;
use crate::user_model::{authenticate_password, UserModel, UserStore};
use doido_core::Result;
use doido_model::password::hash_password;
use doido_model::password::HasSecurePassword;
use std::future::Future;

/// Sign-in using the global [`crate::state::AuthState`] user store (apps/tests).
pub async fn authenticate_global<U>(email: &str, password: &str) -> Result<U, AuthError>
where
    U: UserModel + HasSecurePassword,
{
    let store = crate::state::global().store.clone();
    sign_in(store.as_ref(), email, password).await
}

/// Sign-up using the global user store (apps/tests).
pub async fn register_user_global<U, F, Fut>(
    email: &str,
    password: &str,
    create: F,
) -> Result<U, AuthError>
where
    U: UserModel,
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = Result<U>>,
{
    let store = crate::state::global().store.clone();
    sign_up(store.as_ref(), email, password, create).await
}

fn sign_in_plan() -> FetchPlan {
    let Some(state) = try_global() else {
        return FetchPlan::minimal();
    };
    let config = state.config.clone();
    let mut plan = FetchPlan::for_sign_in(&config);
    for mw in all_middlewares(&config) {
        plan = plan.merge(mw.sign_in_fetch_plan());
    }
    plan
}

pub async fn sign_in<U>(store: &dyn UserStore, email: &str, password: &str) -> Result<U, AuthError>
where
    U: UserModel + HasSecurePassword,
{
    let mws = try_global()
        .map(|s| all_middlewares(&s.config))
        .unwrap_or_default();
    let plan = sign_in_plan();
    let mut ctx = SignInContext::new(store, email, password, &plan);

    for mw in &mws {
        mw.before_sign_in(&mut ctx).await?;
    }

    let row = store.fetch_by_email(email, &plan).await?;
    let row = row.ok_or(AuthError::InvalidCredentials)?;

    lockable::prepare_from_row(store, email, &row).await?;
    confirmable::validate_row(&row)?;

    let user = U::from_row(row.clone()).map_err(|e| AuthError::Internal(e.to_string()))?;
    ctx.row = Some(row);
    ctx.user_id = Some(serde_json::to_value(user.id()).unwrap_or_default());

    if !authenticate_password(&user, password) {
        for mw in &mws {
            mw.after_sign_in_failed(&ctx).await?;
        }
        return Err(AuthError::InvalidCredentials);
    }

    for mw in &mws {
        mw.after_sign_in(&ctx).await?;
        mw.after_sign_in_success(&ctx).await?;
    }
    Ok(user)
}

pub async fn sign_up<U, F, Fut>(
    store: &dyn UserStore,
    email: &str,
    password: &str,
    create: F,
) -> Result<U, AuthError>
where
    U: UserModel,
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = Result<U>>,
{
    let mws = try_global()
        .map(|s| all_middlewares(&s.config))
        .unwrap_or_default();
    let vctx = ValidateContext { email, password };
    for mw in &mws {
        mw.before_validate(&vctx).await?;
    }
    let sctx = SignUpContext {
        store,
        email,
        password,
    };
    for mw in &mws {
        mw.before_sign_up(&sctx).await?;
    }

    if U::find_by_email(store, email, &FetchPlan::minimal())
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?
        .is_some()
    {
        for mw in &mws {
            mw.after_sign_up_failed(&sctx).await?;
        }
        return Err(AuthError::EmailTaken);
    }

    let digest = hash_password(password).map_err(|e| AuthError::Internal(e.to_string()))?;
    match create(email.to_string(), digest).await {
        Ok(user) => {
            for mw in &mws {
                mw.after_sign_up_success(&sctx).await?;
                mw.after_sign_up(&sctx).await?;
            }
            Ok(user)
        }
        Err(e) => {
            for mw in &mws {
                mw.after_sign_up_failed(&sctx).await?;
            }
            Err(AuthError::Internal(e.to_string()))
        }
    }
}

pub async fn sign_out_pipeline(
    store: &dyn UserStore,
    ctx: &mut doido_controller::Context,
    email: Option<String>,
) -> Result<(), AuthError> {
    let mws = try_global()
        .map(|s| all_middlewares(&s.config))
        .unwrap_or_default();
    let mut sctx = SignOutContext { store, ctx, email };
    for mw in &mws {
        mw.before_sign_out(&mut sctx).await?;
    }
    crate::strategy::session::sign_out_session(sctx.ctx.session());
    for mw in &mws {
        mw.after_sign_out_success(&sctx).await?;
    }
    Ok(())
}

pub fn complete_session_sign_in<U: UserModel>(
    session: &mut doido_controller::session::Session,
    user: &U,
) {
    crate::strategy::session::sign_in_session(session, user.id());
}

/// Persist the authenticated user on the request [`doido_controller::Context`] session.
pub fn sign_in_context(
    ctx: &mut doido_controller::Context,
    user: &impl UserModel,
) -> Result<(), AuthError> {
    complete_session_sign_in(ctx.session(), user);
    Ok(())
}

/// Run sign-out middleware hooks and clear the session.
pub async fn sign_out_context(
    ctx: &mut doido_controller::Context,
    email: Option<String>,
) -> Result<(), AuthError> {
    let store = crate::state::global().store.clone();
    sign_out_pipeline(store.as_ref(), ctx, email).await
}
