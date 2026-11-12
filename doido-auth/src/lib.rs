//! Unified authentication for Doido — Devise + OmniAuth + JWT analogue.

pub mod config;
pub mod controllers;
pub mod error;
pub mod extractors;
pub mod framework_views;
pub mod handlers;
pub mod i18n;
pub mod identity;
pub mod layer;
pub mod messages;
pub mod middleware;
pub mod oauth;
pub mod route_mount;
pub mod state;
pub mod strategy;
pub mod user;
pub mod user_model;

pub mod testing;

#[cfg(feature = "auth-2fa")]
pub mod two_factor;

pub mod confirmable {
    pub use crate::middleware::confirmable::*;
}
pub mod lockable {
    pub use crate::middleware::lockable::*;
}
pub mod recoverable {
    pub use crate::middleware::recoverable::*;
}
pub mod trackable {
    pub use crate::middleware::trackable::*;
}
pub mod validations {
    pub use crate::middleware::validatable::*;
}

#[cfg(feature = "auth-2fa")]
pub use controllers::AuthTwoFactor;

pub use config::{
    load as load_config, AuthConfig, AuthModule, AuthRoutesConfig, JwtConfig, UserStoreConfig,
    UserStoreType, YamlConfig,
};
pub use controllers::{AuthOauth, AuthPasswords, AuthRegistrations, AuthSessions};
pub use doido_auth_macros::{__auth_routes_decl, auth_controller, auth_routes, routes};
pub use error::AuthError;
pub use extractors::{AuthToken, CurrentUser, MaybeUser, RequireAuth};
pub use framework_views::register_views;
pub use handlers::{authenticate, register_user, sign_in, sign_in_with_session, sign_out};
pub use i18n::register_locales;
pub use identity::AuthIdentity;
pub use layer::{auth_layer, current_identity, current_user};
pub use middleware::{
    register_middleware, AuthMiddleware, RememberableMiddleware, REMEMBER_COOKIE,
};
pub use oauth::{OAuth2Provider, OAuthProvider, OAuthTokenResponse};
pub use route_mount::mount;
pub use state::{global, init, set_state, try_global, AuthState};
pub use strategy::jwt::{JwtClaims, JwtStrategy, TokenPair};
pub use strategy::registry::{register_strategy, registered_strategies};
pub use strategy::session::{
    SessionStrategy, SessionStrategy as CookieStrategy, SESSION_COOKIE, USER_ID_KEY,
};
pub use strategy::AuthStrategy;
pub use user::{authenticate_password, RegisterableUserModel, UserModel};
pub use user_model::{
    build_user_store, find_by_email_mapped, find_by_id_mapped, FetchContribution, FetchPlan,
    SeaOrmUserStore, SignInRow, UserField, UserPatchBuilder, UserRow, UserStore, UserStoreAction,
};

#[cfg(feature = "auth-2fa")]
pub use two_factor::{
    enroll as enroll_two_factor, verify_code as verify_two_factor_code, TwoFactorEnrollment,
};

#[cfg(feature = "generators")]
pub mod generators;
