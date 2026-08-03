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
    #[serde(default = "default_strategies")]
    pub strategies: Vec<String>,
    #[serde(default)]
    pub jwt: Option<JwtConfig>,
    #[serde(default)]
    pub oauth: HashMap<String, OAuthProviderConfig>,
    #[serde(default)]
    pub two_factor: TwoFactorConfig,
    #[serde(default)]
    pub routes: AuthRoutesConfig,
}

fn default_strategies() -> Vec<String> {
    vec!["cookie".into()]
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            user_model: None,
            strategies: default_strategies(),
            jwt: None,
            oauth: HashMap::new(),
            two_factor: TwoFactorConfig::default(),
            routes: AuthRoutesConfig::default(),
        }
    }
}

impl AuthConfig {
    /// Parse from a YAML string containing an `auth:` section (other sections ignored).
    pub fn from_yaml(yaml: &str) -> Result<Self, std::io::Error> {
        YamlConfig::from_yaml(yaml).map(|c| c.auth)
    }

    /// Validate required secrets and strategy-specific settings.
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
