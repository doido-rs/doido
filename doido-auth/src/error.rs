//! Typed auth errors (`thiserror` per crate, per the framework convention).

/// Errors raised by the auth layer.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Email/password combination is wrong or the account has no password.
    #[error("auth: invalid credentials")]
    InvalidCredentials,

    /// Registration attempted with an email that is already taken.
    #[error("auth: email already taken")]
    EmailTaken,

    /// A `validatable`-module check failed (email format, password length, …).
    #[error("auth: {0}")]
    Validation(String),

    /// Sign-in blocked because the account's email is not confirmed
    /// (`confirmable` module).
    #[error("auth: email not confirmed")]
    NotConfirmed,

    /// Sign-in blocked because the account is locked (`lockable` module).
    #[error("auth: account locked")]
    AccountLocked,

    /// No authenticated identity was found on the request.
    #[error("auth: unauthorized")]
    Unauthorized,

    /// A bearer or JWT token is missing or malformed.
    #[error("auth: invalid token")]
    InvalidToken,

    /// JWT verification failed (wrong secret, expired, etc.).
    #[error("auth: jwt error: {0}")]
    Jwt(String),

    /// OAuth provider or callback failed.
    #[error("auth: oauth error: {0}")]
    OAuth(String),

    /// Two-factor verification failed.
    #[cfg(feature = "auth-2fa")]
    #[error("auth: two-factor error: {0}")]
    TwoFactor(String),

    /// The `auth` configuration is invalid or incomplete.
    #[error("auth: config error: {0}")]
    Config(String),

    /// An unknown auth strategy was referenced in config.
    #[error("auth: unknown strategy: {0}")]
    UnknownStrategy(String),

    /// Database or internal failure.
    #[error("auth: internal error: {0}")]
    Internal(String),
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AuthError::Jwt(e.to_string())
    }
}
