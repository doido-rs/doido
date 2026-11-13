//! Registrable Devise-style auth middleware pipeline.

pub mod confirmable;
pub mod context;
pub mod lockable;
pub mod omniauthable;
pub mod pipeline;
pub mod recoverable;
pub mod registry;
pub mod rememberable;
pub mod timeoutable;
pub mod trackable;
mod trait_;
#[cfg(feature = "auth-2fa")]
pub mod two_factorable;
pub mod validatable;

pub use confirmable::{
    confirm, generate_confirmation, is_confirmed, is_enabled, send_confirmation_email,
    ConfirmableMiddleware,
};
pub use lockable::{ensure_not_locked, record_failure, reset_attempts, LockableMiddleware};
pub use omniauthable::{
    authorize_url, exchange_code, get_provider, is_enabled as omniauthable_enabled,
    providers_from_config, register_provider, resolve_provider, OAuth2Provider, OAuthProvider,
    OAuthTokenResponse, OmniauthableMiddleware,
};
pub use pipeline::{
    authenticate_global, complete_session_sign_in, register_user_global,
    sign_in as pipeline_sign_in, sign_in_context, sign_out_context, sign_out_pipeline,
    sign_up as pipeline_sign_up,
};
pub use recoverable::{request_reset, reset_password, send_reset_email, RecoverableMiddleware};
pub use registry::{built_in_middlewares, register_middleware, reset_custom_middleware};
pub use rememberable::{
    cookie_value, forget, record_remember, RememberableMiddleware, REMEMBER_COOKIE,
};
pub use timeoutable::{is_session_expired, TimeoutableMiddleware};
pub use trackable::{record_sign_in, TrackableMiddleware};
pub use trait_::AuthMiddleware;
#[cfg(feature = "auth-2fa")]
pub use two_factorable::{
    disable_for_user, enable_for_user, enroll, verify_code, verify_user_code, HasTwoFactor,
    TwoFactor, TwoFactorEnrollment, TwoFactorableMiddleware,
};
pub use validatable::{
    validate_email, validate_password_length, validate_registration, ValidatableMiddleware,
};
