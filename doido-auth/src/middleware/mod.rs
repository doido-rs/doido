//! Registrable Devise-style auth middleware pipeline.

pub mod confirmable;
pub mod context;
pub mod lockable;
pub mod pipeline;
pub mod recoverable;
pub mod registry;
pub mod rememberable;
pub mod timeoutable;
pub mod trackable;
mod trait_;
pub mod validatable;

pub use confirmable::{
    confirm, generate_confirmation, is_confirmed, is_enabled, send_confirmation_email,
    ConfirmableMiddleware,
};
pub use lockable::{ensure_not_locked, record_failure, reset_attempts, LockableMiddleware};
pub use pipeline::{
    complete_session_sign_in, sign_in as pipeline_sign_in, sign_out_pipeline,
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
pub use validatable::{
    validate_email, validate_password_length, validate_registration, ValidatableMiddleware,
};
