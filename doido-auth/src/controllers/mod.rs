//! Default auth controllers shipped with `doido-auth`.
//!
//! Apps can point `auth_routes!(User)` at these types directly, or supply custom
//! controllers via `controllers:` / `actions:` options (Devise-style). Generated
//! apps from `auth:install` emit project-local controllers under
//! `app/controllers/auth/` that mirror these defaults and can be extended.

pub mod oauth;
pub mod passwords;
pub mod registrations;
pub mod sessions;

#[cfg(feature = "auth-2fa")]
pub mod two_factor;

pub use oauth::AuthOauth;
pub use passwords::AuthPasswords;
pub use registrations::AuthRegistrations;
pub use sessions::AuthSessions;

#[cfg(feature = "auth-2fa")]
pub use two_factor::AuthTwoFactor;
