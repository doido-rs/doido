//! `auth:` section of `config/<env>.yml` → [`AuthConfig`].

use crate::error::AuthError;
use doido_core::Environment;
use serde::Deserialize;
use std::collections::HashMap;

/// Which auth strategies are enabled (consulted in order by extractors/layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StrategyKind {
    #[default]
    Cookie,
    Jwt,
}

/// Devise-style auth modules, declared under `auth.modules` in `config/<env>.yml`.
///
/// A module toggles a coherent feature — its routes (via the generated
/// `auth_routes!` `only:` list), its migration columns, and its runtime behavior.
/// `strategies` (cookie/jwt) are orthogonal: they decide *how* a request is
/// authenticated, while modules decide *which* Devise features are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthModule {
    /// Password authentication (email + `password_digest`). Effectively required.
    DatabaseAuthenticatable,
    /// Sign-up / account registration (`registrations` routes).
    Registerable,
    /// Password reset via emailed token (`passwords` routes).
    Recoverable,
    /// "Remember me" persistent cookie (`remember_created_at`).
    Rememberable,
    /// Sign-in tracking (count, timestamps, IPs).
    Trackable,
    /// Idle session expiry after `auth.timeout` seconds.
    Timeoutable,
    /// Email/password format + length validation on registration.
    Validatable,
    /// Email confirmation before sign-in (`confirmation` routes).
    Confirmable,
    /// Lock an account after repeated failed sign-ins (`unlock` routes).
    Lockable,
    /// OAuth / social sign-in (`oauth` routes).
    Omniauthable,
    /// TOTP two-factor authentication (requires the `auth-2fa` feature).
    TwoFactorAuthenticatable,
}

impl AuthModule {
    /// All modules, in declaration order.
    pub const ALL: [AuthModule; 11] = [
        AuthModule::DatabaseAuthenticatable,
        AuthModule::Registerable,
        AuthModule::Recoverable,
        AuthModule::Rememberable,
        AuthModule::Trackable,
        AuthModule::Timeoutable,
        AuthModule::Validatable,
        AuthModule::Confirmable,
        AuthModule::Lockable,
        AuthModule::Omniauthable,
        AuthModule::TwoFactorAuthenticatable,
    ];

    /// The snake_case name used in config and generator flags.
    pub fn as_str(self) -> &'static str {
        match self {
            AuthModule::DatabaseAuthenticatable => "database_authenticatable",
            AuthModule::Registerable => "registerable",
            AuthModule::Recoverable => "recoverable",
            AuthModule::Rememberable => "rememberable",
            AuthModule::Trackable => "trackable",
            AuthModule::Timeoutable => "timeoutable",
            AuthModule::Validatable => "validatable",
            AuthModule::Confirmable => "confirmable",
            AuthModule::Lockable => "lockable",
            AuthModule::Omniauthable => "omniauthable",
            AuthModule::TwoFactorAuthenticatable => "two_factor_authenticatable",
        }
    }

    /// Parse a module from its snake_case name.
    pub fn from_str(s: &str) -> Option<AuthModule> {
        AuthModule::ALL.into_iter().find(|m| m.as_str() == s)
    }

    /// The `auth_routes!` route-group name this module mounts, if any.
    /// Behavior-only modules (trackable, timeoutable, rememberable, validatable,
    /// database_authenticatable) return `None` — they add no dedicated routes.
    pub fn route_group(self) -> Option<&'static str> {
        match self {
            AuthModule::Registerable => Some("registrations"),
            AuthModule::Recoverable => Some("passwords"),
            AuthModule::Confirmable => Some("confirmation"),
            AuthModule::Lockable => Some("unlock"),
            AuthModule::Omniauthable => Some("oauth"),
            AuthModule::TwoFactorAuthenticatable => Some("two_factor"),
            _ => None,
        }
    }
}

/// JWT bearer settings from the `auth.jwt` section.
#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    #[serde(default = "default_access_ttl")]
    pub access_ttl: u64,
    #[serde(default = "default_refresh_ttl")]
    pub refresh_ttl: u64,
    #[serde(default)]
    pub issuer: Option<String>,
}

fn default_access_ttl() -> u64 {
    900
}

fn default_refresh_ttl() -> u64 {
    604_800
}

impl JwtConfig {
    pub fn validate(&self) -> Result<(), AuthError> {
        if self.secret.trim().is_empty() {
            return Err(AuthError::Config(
                "auth.jwt.secret must not be empty".into(),
            ));
        }
        Ok(())
    }
}

/// OAuth provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProviderType {
    Oauth1,
    Oauth2,
}

/// One OAuth/OAuth2 provider entry under `auth.oauth`.
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: OAuthProviderType,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub consumer_key: Option<String>,
    #[serde(default)]
    pub consumer_secret: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub authorize_url: Option<String>,
    #[serde(default)]
    pub token_url: Option<String>,
}

/// Two-factor settings from `auth.two_factor`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TwoFactorConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub issuer: Option<String>,
}

/// Devise-style route prefix and path segments.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthRoutesConfig {
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default = "default_sign_in")]
    pub sign_in: String,
    #[serde(default = "default_sign_out")]
    pub sign_out: String,
    #[serde(default = "default_sign_up")]
    pub sign_up: String,
    #[serde(default = "default_password_reset")]
    pub password_reset: String,
}

fn default_prefix() -> String {
    "/users".into()
}

fn default_sign_in() -> String {
    "sign_in".into()
}

fn default_sign_out() -> String {
    "sign_out".into()
}

fn default_sign_up() -> String {
    "sign_up".into()
}

fn default_password_reset() -> String {
    "password".into()
}

impl Default for AuthRoutesConfig {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            sign_in: default_sign_in(),
            sign_out: default_sign_out(),
            sign_up: default_sign_up(),
            password_reset: default_password_reset(),
        }
    }
}

impl AuthRoutesConfig {
    pub fn sign_in_path(&self) -> String {
        format!("{}/{}", self.prefix.trim_end_matches('/'), self.sign_in)
    }

    pub fn sign_out_path(&self) -> String {
        format!("{}/{}", self.prefix.trim_end_matches('/'), self.sign_out)
    }

    pub fn sign_up_path(&self) -> String {
        format!("{}/{}", self.prefix.trim_end_matches('/'), self.sign_up)
    }

    pub fn password_path(&self) -> String {
        format!(
            "{}/{}",
            self.prefix.trim_end_matches('/'),
            self.password_reset
        )
    }
}

/// Full auth configuration deserialized from the `auth` section.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub user_model: Option<String>,
    #[serde(default = "default_modules")]
    pub modules: Vec<AuthModule>,
    #[serde(default = "default_strategies")]
    pub strategies: Vec<String>,
    #[serde(default)]
    pub jwt: Option<JwtConfig>,
    #[serde(default)]
    pub oauth: HashMap<String, OAuthProviderConfig>,
    #[serde(default)]
    pub two_factor: TwoFactorConfig,
    /// Idle-session timeout in seconds for the `timeoutable` module.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Minimum password length enforced by the `validatable` module.
    #[serde(default = "default_password_length")]
    pub password_length: usize,
    /// Failed sign-in attempts before `lockable` locks an account.
    #[serde(default = "default_maximum_attempts")]
    pub maximum_attempts: u32,
    /// Seconds a `lockable` account stays locked before auto-unlocking.
    #[serde(default = "default_unlock_in")]
    pub unlock_in: i64,
    /// Seconds a `recoverable` password-reset token stays valid.
    #[serde(default = "default_reset_within")]
    pub reset_password_within: i64,
    /// Seconds a `rememberable` "remember me" cookie persists.
    #[serde(default = "default_remember_for")]
    pub remember_for: i64,
    #[serde(default)]
    pub routes: AuthRoutesConfig,
}

fn default_strategies() -> Vec<String> {
    vec!["cookie".into()]
}

/// Devise's default module set for a generated model.
fn default_modules() -> Vec<AuthModule> {
    vec![
        AuthModule::DatabaseAuthenticatable,
        AuthModule::Registerable,
        AuthModule::Recoverable,
        AuthModule::Rememberable,
        AuthModule::Validatable,
    ]
}

fn default_timeout() -> u64 {
    1_800
}

fn default_password_length() -> usize {
    6
}

fn default_maximum_attempts() -> u32 {
    20
}

fn default_unlock_in() -> i64 {
    3_600
}

fn default_reset_within() -> i64 {
    21_600
}

fn default_remember_for() -> i64 {
    1_209_600
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            user_model: None,
            modules: default_modules(),
            strategies: default_strategies(),
            jwt: None,
            oauth: HashMap::new(),
            two_factor: TwoFactorConfig::default(),
            timeout: default_timeout(),
            password_length: default_password_length(),
            maximum_attempts: default_maximum_attempts(),
            unlock_in: default_unlock_in(),
            reset_password_within: default_reset_within(),
            remember_for: default_remember_for(),
            routes: AuthRoutesConfig::default(),
        }
    }
}

impl AuthConfig {
    /// Parse from a YAML string containing an `auth:` section (other sections ignored).
    pub fn from_yaml(yaml: &str) -> Result<Self, std::io::Error> {
        YamlConfig::from_yaml(yaml).map(|c| c.auth)
    }

    /// Returns whether `module` is enabled.
    pub fn has_module(&self, module: AuthModule) -> bool {
        self.modules.contains(&module)
    }

    /// The `auth_routes!` route-group names for the enabled modules, in a stable
    /// order (`sessions` is always present for `database_authenticatable`). Used
    /// to generate the `only:` list and to gate the runtime route mounter.
    pub fn enabled_route_groups(&self) -> Vec<&'static str> {
        let mut groups = vec!["sessions"];
        for module in AuthModule::ALL {
            if self.has_module(module) {
                if let Some(group) = module.route_group() {
                    groups.push(group);
                }
            }
        }
        groups
    }

    /// Validate required secrets, strategy-specific settings, and module coherence.
    pub fn validate(&self) -> Result<(), AuthError> {
        for name in &self.strategies {
            match name.as_str() {
                "cookie" => {}
                "jwt" => {
                    let jwt = self.jwt.as_ref().ok_or_else(|| {
                        AuthError::Config(
                            "auth.jwt section required when jwt strategy is enabled".into(),
                        )
                    })?;
                    jwt.validate()?;
                }
                other => {
                    if !crate::registry::has_strategy(other) {
                        return Err(AuthError::UnknownStrategy(other.to_string()));
                    }
                }
            }
        }

        if !self.has_module(AuthModule::DatabaseAuthenticatable) {
            return Err(AuthError::Config(
                "auth.modules must include database_authenticatable".into(),
            ));
        }
        if self.has_module(AuthModule::TwoFactorAuthenticatable) && !cfg!(feature = "auth-2fa") {
            return Err(AuthError::Config(
                "auth.modules includes two_factor_authenticatable but the `auth-2fa` feature is not enabled".into(),
            ));
        }

        Ok(())
    }

    pub fn strategy_kinds(&self) -> Vec<StrategyKind> {
        self.strategies
            .iter()
            .filter_map(|s| match s.as_str() {
                "cookie" => Some(StrategyKind::Cookie),
                "jwt" => Some(StrategyKind::Jwt),
                _ => None,
            })
            .collect()
    }
}

/// File-based config wrapper — only the `auth` section is read.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct YamlConfig {
    #[serde(default)]
    pub auth: AuthConfig,
}

impl YamlConfig {
    pub fn load() -> std::io::Result<Self> {
        Self::load_env(Environment::get_env())
    }

    pub fn load_env(env: Environment) -> std::io::Result<Self> {
        let path = format!("config/{}.yml", env.as_str());
        let contents = std::fs::read_to_string(&path)?;
        Self::from_yaml(&contents)
    }

    pub fn from_yaml(yaml: &str) -> std::io::Result<Self> {
        serde_norway::from_str(yaml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Loads the current environment's [`AuthConfig`], defaulting when missing.
pub fn load() -> AuthConfig {
    YamlConfig::load().map(|c| c.auth).unwrap_or_default()
}
