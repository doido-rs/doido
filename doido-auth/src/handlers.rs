//! Sign-in, sign-up, and sign-out helpers built on [`UserModel`] and the auth middleware pipeline.

use crate::error::AuthError;
use crate::middleware::{
    complete_session_sign_in, pipeline_sign_in, pipeline_sign_up, sign_out_pipeline,
};
use crate::state::global;
use crate::user::UserModel;
use doido_controller::session::Session;
use doido_controller::Context;
use doido_core::Result;
use doido_model::password::HasSecurePassword;
use std::future::Future;

fn store() -> std::sync::Arc<dyn crate::user_model::UserStore> {
    global().store.clone()
}

/// Authenticate by email/password and return the user (single validation fetch).
pub async fn authenticate<U>(email: &str, password: &str) -> Result<U, AuthError>
where
    U: UserModel + HasSecurePassword,
{
    pipeline_sign_in(store().as_ref(), email, password).await
}

/// Persist `user_id` in the encrypted session cookie payload.
pub fn sign_in_with_session<U: UserModel>(session: &mut Session, user: &U) {
    complete_session_sign_in(session, user);
}

/// Establish an authenticated session for `user` on the request context.
pub fn sign_in(ctx: &mut Context, user: &impl UserModel) -> Result<(), AuthError> {
    sign_in_with_session(ctx.session(), user);
    Ok(())
}

/// Clear the authenticated user from the session on the request context.
pub async fn sign_out(ctx: &mut Context, email: Option<String>) -> Result<(), AuthError> {
    sign_out_pipeline(store().as_ref(), ctx, email).await
}

/// Register a new user: validation middleware, uniqueness check, then `create`.
pub async fn register_user<U, F, Fut>(
    email: &str,
    password: &str,
    create: F,
) -> Result<U, AuthError>
where
    U: UserModel,
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = Result<U>>,
{
    pipeline_sign_up(store().as_ref(), email, password, create).await
}
