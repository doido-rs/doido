//! OAuth provider registry — abstract interface + config-driven OAuth 2.0 impl.

use crate::config::{OAuthProviderConfig, OAuthProviderType};
use crate::error::AuthError;
use doido_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Token payload returned after a successful authorization-code exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub id_token: Option<String>,
}

/// Pluggable OAuth provider — apps register custom impls or use config-backed ones.
pub trait OAuthProvider: Send + Sync {
    /// Registry key (matches the `:provider` route segment and config entry name).
    fn name(&self) -> &str;

    /// Build the authorization redirect URL for the given CSRF `state`.
    fn authorize_url(&self, state: &str) -> Result<String, AuthError>;

    /// Exchange an authorization `code` for tokens.
    fn exchange_code(&self, code: &str) -> Result<OAuthTokenResponse, AuthError>;
}

/// Config-driven OAuth 2.0 authorization-code provider.
pub struct OAuth2Provider {
    name: String,
    config: OAuthProviderConfig,
}

impl OAuth2Provider {
    pub fn new(name: impl Into<String>, config: OAuthProviderConfig) -> Self {
        Self {
            name: name.into(),
            config,
        }
    }

    pub fn from_config(
        name: impl Into<String>,
        config: OAuthProviderConfig,
    ) -> Result<Self, AuthError> {
        let name = name.into();
        if config.provider_type != OAuthProviderType::Oauth2 {
            return Err(AuthError::OAuth(format!("provider {name} is not oauth2")));
        }
        Ok(Self::new(name, config))
    }
}

impl OAuthProvider for OAuth2Provider {
    fn name(&self) -> &str {
        &self.name
    }

    fn authorize_url(&self, state: &str) -> Result<String, AuthError> {
        let client_id = self
            .config
            .client_id
            .as_deref()
            .ok_or_else(|| AuthError::OAuth("missing client_id".into()))?;
        let authorize_url = self
            .config
            .authorize_url
            .as_deref()
            .ok_or_else(|| AuthError::OAuth("missing authorize_url".into()))?;
        let redirect_uri = self
            .config
            .redirect_uri
            .as_deref()
            .ok_or_else(|| AuthError::OAuth("missing redirect_uri".into()))?;

        let scope = if self.config.scopes.is_empty() {
            String::new()
        } else {
            format!("&scope={}", url_encode(&self.config.scopes.join(" ")))
        };
        Ok(format!(
            "{authorize_url}?client_id={}&redirect_uri={}&response_type=code&state={}{scope}",
            url_encode(client_id),
            url_encode(redirect_uri),
            url_encode(state),
        ))
    }

    fn exchange_code(&self, code: &str) -> Result<OAuthTokenResponse, AuthError> {
        let token_url = self
            .config
            .token_url
            .as_deref()
            .ok_or_else(|| AuthError::OAuth("missing token_url".into()))?;
        let client_id = self
            .config
            .client_id
            .as_deref()
            .ok_or_else(|| AuthError::OAuth("missing client_id".into()))?;
        let client_secret = self
            .config
            .client_secret
            .as_deref()
            .ok_or_else(|| AuthError::OAuth("missing client_secret".into()))?;
        let redirect_uri = self
            .config
            .redirect_uri
            .as_deref()
            .ok_or_else(|| AuthError::OAuth("missing redirect_uri".into()))?;

        let body = format!(
            "grant_type=authorization_code&code={code}&redirect_uri={redirect_uri}&client_id={client_id}&client_secret={client_secret}"
        );
        let response = ureq::post(token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(body)
            .map_err(|e| AuthError::OAuth(format!("token exchange failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.into_body().read_to_string().unwrap_or_default();
            return Err(AuthError::OAuth(format!(
                "token exchange HTTP {status}: {text}"
            )));
        }

        response
            .into_body()
            .read_json::<OAuthTokenResponse>()
            .map_err(|e| AuthError::OAuth(format!("invalid token response: {e}")))
    }
}

static PROVIDERS: OnceLock<RwLock<HashMap<String, Arc<dyn OAuthProvider>>>> = OnceLock::new();

fn providers() -> &'static RwLock<HashMap<String, Arc<dyn OAuthProvider>>> {
    PROVIDERS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a custom OAuth provider at boot.
pub fn register_provider(provider: Arc<dyn OAuthProvider>) {
    providers()
        .write()
        .expect("oauth provider lock")
        .insert(provider.name().to_string(), provider);
}

/// Look up a registered OAuth provider by name.
pub fn get_provider(name: &str) -> Option<Arc<dyn OAuthProvider>> {
    providers()
        .read()
        .expect("oauth provider lock")
        .get(name)
        .cloned()
}

/// Build providers from config entries (OAuth 2.0 only in v1).
pub fn providers_from_config(
    oauth: &HashMap<String, OAuthProviderConfig>,
) -> HashMap<String, Arc<dyn OAuthProvider>> {
    let mut map = HashMap::new();
    for (name, cfg) in oauth {
        if cfg.provider_type == OAuthProviderType::Oauth2 {
            if let Ok(provider) = OAuth2Provider::from_config(name, cfg.clone()) {
                map.insert(name.clone(), Arc::new(provider) as Arc<dyn OAuthProvider>);
            }
        }
    }
    map
}

fn url_encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
