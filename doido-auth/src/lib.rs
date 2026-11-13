//! Unified authentication for Doido — Devise + OmniAuth + JWT analogue.

pub mod config;
pub mod controllers;
pub mod error;
pub mod extractors;
pub mod framework_views;
pub mod i18n;
pub mod layer;
pub mod middleware;
pub mod route_mount;
pub mod state;
pub mod strategy;
pub mod user;
pub mod user_model;

pub mod testing;

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
pub mod omniauthable {
    pub use crate::middleware::omniauthable::*;
}
/// Back-compat re-exports — prefer [`omniauthable`].
pub mod oauth {
    pub use crate::middleware::omniauthable::{
        get_provider, providers_from_config, register_provider, OAuth2Provider, OAuthProvider,
        OAuthTokenResponse,
    };
}
#[cfg(feature = "auth-2fa")]
pub mod two_factorable {
    pub use crate::middleware::two_factorable::*;
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
pub use controllers::messages;
pub use controllers::{AuthOauth, AuthPasswords, AuthRegistrations, AuthSessions};
pub use doido_auth_macros::{__auth_routes_decl, auth_controller, auth_routes, routes};
pub use error::AuthError;
pub use extractors::{AuthToken, CurrentUser, MaybeUser, RequireAuth};
pub use framework_views::register_views;
pub use i18n::register_locales;
pub use layer::{auth_layer, current_identity, current_user};
pub use middleware::omniauthable::{OAuth2Provider, OAuthProvider, OAuthTokenResponse};
pub use middleware::{
    authenticate_global as authenticate, complete_session_sign_in as sign_in_with_session,
    register_middleware, register_user_global as register_user, sign_in_context as sign_in,
    sign_out_context as sign_out, AuthMiddleware, OmniauthableMiddleware, RememberableMiddleware,
    REMEMBER_COOKIE,
};
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
    build_user_store, find_by_email_mapped, find_by_id_mapped, AuthIdentity, FetchContribution,
    FetchPlan, SeaOrmUserStore, SignInRow, UserField, UserPatchBuilder, UserRow, UserStore,
    UserStoreAction,
};

#[cfg(feature = "auth-2fa")]
pub use two_factorable::{
    enroll as enroll_two_factor, verify_code as verify_two_factor_code, HasTwoFactor, TwoFactor,
    TwoFactorEnrollment, TwoFactorableMiddleware,
};

#[cfg(feature = "generators")]
pub mod generators;
