//! Storage-neutral user record returned by [`super::UserStore`].

use super::plan::UserField;
use serde_json::Value;

/// Raw user row from any backend — middleware validates using these fields.
#[derive(Debug, Clone, Default)]
pub struct UserRow {
    pub id: Option<Value>,
    pub email: Option<String>,
    pub password_digest: Option<String>,
    pub failed_attempts: Option<i32>,
    pub locked_at: Option<String>,
    pub confirmed_at: Option<String>,
    pub confirmation_token: Option<String>,
    pub confirmation_sent_at: Option<String>,
    pub sign_in_count: Option<i32>,
    pub current_sign_in_at: Option<String>,
    pub last_sign_in_at: Option<String>,
    pub current_sign_in_ip: Option<String>,
    pub last_sign_in_ip: Option<String>,
    pub remember_created_at: Option<String>,
    pub reset_password_token: Option<String>,
    pub reset_password_sent_at: Option<String>,
    pub unlock_token: Option<String>,
    pub unconfirmed_email: Option<String>,
    pub two_factor_secret: Option<String>,
    pub two_factor_enabled: Option<bool>,
}

impl UserRow {
    pub fn set_from_column(&mut self, field: UserField, value: Option<String>) {
        match field {
            UserField::Id => {
                self.id =
                    value.and_then(|s| serde_json::from_str(&s).ok().or(Some(Value::String(s))));
            }
            UserField::Email => self.email = value,
            UserField::PasswordDigest => self.password_digest = value,
            UserField::FailedAttempts => {
                self.failed_attempts = value.and_then(|s| s.parse().ok());
            }
            UserField::LockedAt => self.locked_at = value,
            UserField::ConfirmedAt => self.confirmed_at = value,
            UserField::ConfirmationToken => self.confirmation_token = value,
            UserField::ConfirmationSentAt => self.confirmation_sent_at = value,
            UserField::SignInCount => {
                self.sign_in_count = value.and_then(|s| s.parse().ok());
            }
            UserField::CurrentSignInAt => self.current_sign_in_at = value,
            UserField::LastSignInAt => self.last_sign_in_at = value,
            UserField::CurrentSignInIp => self.current_sign_in_ip = value,
            UserField::LastSignInIp => self.last_sign_in_ip = value,
            UserField::RememberCreatedAt => self.remember_created_at = value,
            UserField::ResetPasswordToken => self.reset_password_token = value,
            UserField::ResetPasswordSentAt => self.reset_password_sent_at = value,
            UserField::UnlockToken => self.unlock_token = value,
            UserField::UnconfirmedEmail => self.unconfirmed_email = value,
            UserField::TwoFactorSecret => self.two_factor_secret = value,
            UserField::TwoFactorEnabled => {
                self.two_factor_enabled = value.as_deref().map(parse_bool);
            }
        }
    }

    pub fn set_id_json(&mut self, id: Value) {
        self.id = Some(id);
    }
}

fn parse_bool(s: &str) -> bool {
    matches!(s.trim(), "1" | "true" | "t" | "yes" | "on")
}

pub type SignInRow = UserRow;
