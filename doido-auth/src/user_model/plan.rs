//! Composable column sets for a single user fetch.

use crate::config::{AuthConfig, AuthModule};
use std::collections::HashSet;

/// Conventional user-table columns that auth middleware may need on one read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserField {
    Id,
    Email,
    PasswordDigest,
    FailedAttempts,
    LockedAt,
    ConfirmedAt,
    ConfirmationToken,
    ConfirmationSentAt,
    SignInCount,
    CurrentSignInAt,
    LastSignInAt,
    CurrentSignInIp,
    LastSignInIp,
    RememberCreatedAt,
    ResetPasswordToken,
    ResetPasswordSentAt,
    UnlockToken,
    UnconfirmedEmail,
    TwoFactorSecret,
    TwoFactorEnabled,
}

impl UserField {
    pub const fn column_name(self) -> &'static str {
        match self {
            UserField::Id => "id",
            UserField::Email => "email",
            UserField::PasswordDigest => "password_digest",
            UserField::FailedAttempts => "failed_attempts",
            UserField::LockedAt => "locked_at",
            UserField::ConfirmedAt => "confirmed_at",
            UserField::ConfirmationToken => "confirmation_token",
            UserField::ConfirmationSentAt => "confirmation_sent_at",
            UserField::SignInCount => "sign_in_count",
            UserField::CurrentSignInAt => "current_sign_in_at",
            UserField::LastSignInAt => "last_sign_in_at",
            UserField::CurrentSignInIp => "current_sign_in_ip",
            UserField::LastSignInIp => "last_sign_in_ip",
            UserField::RememberCreatedAt => "remember_created_at",
            UserField::ResetPasswordToken => "reset_password_token",
            UserField::ResetPasswordSentAt => "reset_password_sent_at",
            UserField::UnlockToken => "unlock_token",
            UserField::UnconfirmedEmail => "unconfirmed_email",
            UserField::TwoFactorSecret => "two_factor_secret",
            UserField::TwoFactorEnabled => "two_factor_enabled",
        }
    }
}

/// Extra columns requested by one [`crate::middleware::AuthMiddleware`].
#[derive(Debug, Clone, Default)]
pub struct FetchContribution {
    pub fields: HashSet<UserField>,
}

impl FetchContribution {
    pub fn with(mut self, field: UserField) -> Self {
        self.fields.insert(field);
        self
    }
}

/// Column set for a single `UserStore::fetch_*` call.
#[derive(Debug, Clone, Default)]
pub struct FetchPlan {
    pub fields: HashSet<UserField>,
}

impl FetchPlan {
    pub fn minimal() -> Self {
        let mut fields = HashSet::new();
        fields.insert(UserField::Id);
        fields.insert(UserField::Email);
        fields.insert(UserField::PasswordDigest);
        Self { fields }
    }

    pub fn for_sign_in(config: &AuthConfig) -> Self {
        let mut plan = Self::minimal();
        if config.has_module(AuthModule::Lockable) {
            plan.fields.insert(UserField::FailedAttempts);
            plan.fields.insert(UserField::LockedAt);
        }
        if config.has_module(AuthModule::Confirmable) {
            plan.fields.insert(UserField::ConfirmedAt);
        }
        #[cfg(feature = "auth-2fa")]
        if config.has_module(AuthModule::TwoFactorAuthenticatable) {
            plan.fields.insert(UserField::TwoFactorSecret);
            plan.fields.insert(UserField::TwoFactorEnabled);
        }
        plan
    }

    pub fn merge(mut self, contribution: FetchContribution) -> Self {
        self.fields.extend(contribution.fields);
        self
    }

    pub fn merge_all(mut self, contributions: impl IntoIterator<Item = FetchContribution>) -> Self {
        for c in contributions {
            self.fields.extend(c.fields);
        }
        self
    }

    pub fn column_list(&self) -> String {
        let mut cols: Vec<&'static str> = self.fields.iter().map(|f| f.column_name()).collect();
        cols.sort_unstable();
        cols.dedup();
        cols.join(", ")
    }
}
