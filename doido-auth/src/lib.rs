//! Unified authentication for Doido — Devise + OmniAuth + JWT analogue.

pub mod config;
pub mod controllers;
pub mod error;
pub mod extractors;
pub mod framework_views;
pub mod handlers;
pub mod identity;
pub mod jwt;
pub mod layer;
pub mod oauth;
pub mod registry;
pub mod route_mount;
pub mod session;
pub mod state;
pub mod strategy;
pub mod user;

pub mod testing;
pub mod validations;

#[cfg(feature = "auth-2fa")]
pub mod two_factor;

#[cfg(feature = "auth-2fa")]
pub use controllers::AuthTwoFactor;

pub use config::{
    load as load_config, AuthConfig, AuthModule, AuthRoutesConfig, JwtConfig, YamlConfig,
};
pub use controllers::{AuthOauth, AuthPasswords, AuthRegistrations, AuthSessions};
pub use doido_auth_macros::{__auth_routes_decl, auth_controller, auth_routes, routes};
pub use error::AuthError;
pub use extractors::{AuthToken, CurrentUser, MaybeUser, RequireAuth};
pub use framework_views::register_views;
pub use handlers::{authenticate, register_user, sign_in, sign_in_with_session, sign_out};
pub use identity::AuthIdentity;
pub use jwt::{JwtClaims, JwtStrategy, TokenPair};
pub use layer::{auth_layer, current_identity, current_user};
pub use oauth::{OAuth2Provider, OAuthProvider, OAuthTokenResponse};
pub use registry::{register_strategy, registered_strategies};
pub use route_mount::mount;
pub use session::{
    SessionStrategy, SessionStrategy as CookieStrategy, SESSION_COOKIE, USER_ID_KEY,
};
pub use state::{global, init, set_state, try_global, AuthState};
pub use strategy::AuthStrategy;
pub use user::{authenticate_password, AuthUser, RegisterableAuthUser};

#[cfg(feature = "auth-2fa")]
pub use two_factor::{
    enroll as enroll_two_factor, verify_code as verify_two_factor_code, TwoFactorEnrollment,
};

#[cfg(feature = "generators")]
pub mod generators;
