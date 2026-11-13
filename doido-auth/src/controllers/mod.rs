//! Default auth controllers shipped with `doido-auth`.
//!
//! Apps can point `auth_routes!(User)` at these types directly, or supply custom
//! controllers via `controllers:` / `actions:` options (Devise-style). Generated
//! apps from `auth:install` emit project-local controllers under
//! `app/controllers/auth/` that mirror these defaults and can be extended.

pub mod confirmations_controller;
pub mod extractors;
pub mod layer;
pub mod messages;
pub mod oauth_controller;
pub mod passwords_controller;
pub mod registrations_controller;
pub mod sessions_controller;

#[cfg(feature = "auth-2fa")]
pub mod two_factor_controller;

pub use confirmations_controller::AuthConfirmations;
pub use extractors::{AuthToken, CurrentUser, MaybeUser, RequireAuth};
pub use layer::{auth_layer, current_identity, current_user};
pub use messages::{json_error, t};
pub use oauth_controller::AuthOauth;
pub use passwords_controller::AuthPasswords;
pub use registrations_controller::AuthRegistrations;
pub use sessions_controller::AuthSessions;

#[cfg(feature = "auth-2fa")]
pub use two_factor_controller::AuthTwoFactor;
