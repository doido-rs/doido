//! Unified authentication for Doido — Devise + OmniAuth + JWT analogue.

pub mod config;
pub mod controllers;
pub mod error;
pub mod framework_views;
pub mod i18n;
pub mod middleware;
pub mod route_mount;
pub mod strategy;
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
/// Back-compat re-exports — prefer [`controllers::layer`].
pub mod layer {
    pub use crate::controllers::layer::{auth_layer, current_identity, current_user};
}
/// Back-compat re-exports — prefer [`controllers::extractors`].
pub mod extractors {
    pub use crate::controllers::extractors::{AuthToken, CurrentUser, MaybeUser, RequireAuth};
}
/// Back-compat re-exports — prefer [`controllers::state`].
pub mod state {
    pub use crate::controllers::state::{
        global, init, reset_state, set_state, try_global, AuthState,
    };
}
/// Back-compat re-exports — prefer [`user_model`].
pub mod user {
    pub use crate::user_model::{
        authenticate_password, find_by_email_mapped, find_by_id_mapped, id_to_sea_value,
        RegisterableUserModel, UserModel,
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
pub use controllers::extractors::{AuthToken, CurrentUser, MaybeUser, RequireAuth};
pub use controllers::layer::{auth_layer, current_identity, current_user};
pub use controllers::messages;
pub use controllers::state::{global, init, set_state, try_global, AuthState};
pub use controllers::{AuthOauth, AuthPasswords, AuthRegistrations, AuthSessions};
pub use doido_auth_macros::{__auth_routes_decl, auth_controller, auth_routes, routes};
pub use error::AuthError;
pub use framework_views::register_views;
pub use i18n::register_locales;
pub use middleware::omniauthable::{OAuth2Provider, OAuthProvider, OAuthTokenResponse};
pub use middleware::{
    authenticate_global as authenticate, complete_session_sign_in as sign_in_with_session,
    register_middleware, register_user_global as register_user, sign_in_context as sign_in,
    sign_out_context as sign_out, AuthMiddleware, OmniauthableMiddleware, RememberableMiddleware,
    REMEMBER_COOKIE,
};
pub use route_mount::mount;
pub use strategy::jwt::{JwtClaims, JwtStrategy, TokenPair};
pub use strategy::registry::{register_strategy, registered_strategies};
pub use strategy::session::{
    SessionStrategy, SessionStrategy as CookieStrategy, SESSION_COOKIE, USER_ID_KEY,
};
pub use strategy::AuthStrategy;
pub use user_model::{
    authenticate_password, build_user_store, find_by_email_mapped, find_by_id_mapped, AuthIdentity,
    FetchContribution, FetchPlan, RegisterableUserModel, SeaOrmUserStore, SignInRow, UserField,
    UserModel, UserPatchBuilder, UserRow, UserStore, UserStoreAction,
};

#[cfg(feature = "auth-2fa")]
pub use two_factorable::{
    enroll as enroll_two_factor, verify_code as verify_two_factor_code, HasTwoFactor, TwoFactor,
    TwoFactorEnrollment, TwoFactorableMiddleware,
};

#[cfg(feature = "generators")]
pub mod generators;
