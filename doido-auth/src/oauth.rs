//! OAuth2 provider registry and token exchange.

use crate::config::{OAuthProviderConfig, OAuthProviderType};
use crate::error::AuthError;
use doido_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// OAuth2 token response from the provider.
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

/// OAuth2 authorization + token exchange provider.
pub struct OAuth2Provider {
    pub name: String,
    pub config: OAuthProviderConfig,
}

impl OAuth2Provider {
    pub fn new(name: impl Into<String>, config: OAuthProviderConfig) -> Self {
        Self {
            name: name.into(),
            config,
        }
    }

    /// Build the provider authorization URL for the OAuth2 authorization-code flow.
    pub fn authorize_url(&self, state: &str) -> Result<String, AuthError> {
        if self.config.provider_type != OAuthProviderType::Oauth2 {
            return Err(AuthError::OAuth(format!(
                "provider {} is not oauth2",
                self.name
            )));
        }
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

    /// Exchange an authorization `code` for tokens at the provider token endpoint.
    pub fn exchange_code(&self, code: &str) -> Result<OAuthTokenResponse, AuthError> {
        if self.config.provider_type != OAuthProviderType::Oauth2 {
            return Err(AuthError::OAuth(format!(
                "provider {} is not oauth2",
                self.name
            )));
        }
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

static PROVIDERS: OnceLock<RwLock<HashMap<String, Arc<OAuth2Provider>>>> = OnceLock::new();

fn providers() -> &'static RwLock<HashMap<String, Arc<OAuth2Provider>>> {
    PROVIDERS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a custom OAuth provider at boot.
pub fn register_provider(name: impl Into<String>, provider: OAuth2Provider) {
    providers()
        .write()
        .expect("oauth provider lock")
        .insert(name.into(), Arc::new(provider));
}

/// Look up a registered OAuth provider.
pub fn get_provider(name: &str) -> Option<Arc<OAuth2Provider>> {
    providers()
        .read()
        .expect("oauth provider lock")
        .get(name)
        .cloned()
}

/// Build providers from config entries (OAuth2 only in v1).
pub fn providers_from_config(
    oauth: &HashMap<String, OAuthProviderConfig>,
) -> HashMap<String, Arc<OAuth2Provider>> {
    let mut map = HashMap::new();
    for (name, cfg) in oauth {
        if cfg.provider_type == OAuthProviderType::Oauth2 {
            map.insert(
                name.clone(),
                Arc::new(OAuth2Provider::new(name, cfg.clone())),
            );
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
