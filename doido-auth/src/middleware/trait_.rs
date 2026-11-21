//! [`AuthMiddleware`] — registrable Devise-style feature hooks.

use super::context::{SignInContext, SignOutContext, SignUpContext, ValidateContext};
use crate::error::AuthError;
use crate::user_model::FetchContribution;
use async_trait::async_trait;

#[async_trait]
pub trait AuthMiddleware: Send + Sync {
    fn name(&self) -> &'static str;

    fn sign_in_fetch_plan(&self) -> FetchContribution {
        FetchContribution::default()
    }

    async fn before_validate(&self, _ctx: &ValidateContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_validate(&self, _ctx: &ValidateContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_validate_success(&self, _ctx: &ValidateContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_validate_failed(&self, _ctx: &ValidateContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn before_sign_in(&self, _ctx: &mut SignInContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_sign_in(&self, _ctx: &SignInContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_sign_in_success(&self, _ctx: &SignInContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_sign_in_failed(&self, _ctx: &SignInContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn before_sign_up(&self, _ctx: &SignUpContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_sign_up(&self, _ctx: &SignUpContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_sign_up_success(&self, _ctx: &SignUpContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_sign_up_failed(&self, _ctx: &SignUpContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn before_sign_out(&self, _ctx: &mut SignOutContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_sign_out_success(&self, _ctx: &SignOutContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }

    async fn after_sign_out_failed(&self, _ctx: &SignOutContext<'_>) -> Result<(), AuthError> {
        Ok(())
    }
}
