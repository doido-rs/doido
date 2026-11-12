//! JWT bearer strategy — sign/verify access and refresh tokens.

use crate::config::JwtConfig;
use crate::error::AuthError;
use crate::identity::AuthIdentity;
use crate::strategy::AuthStrategy;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use doido_core::Result;
use doido_model::sea_orm::DatabaseConnection;
use http::header;
use http::request::Parts;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JWT claims for access and refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Value,
    pub exp: i64,
    pub iat: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
}

/// Issued token pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// JWT bearer strategy.
pub struct JwtStrategy {
    config: JwtConfig,
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtStrategy {
    pub fn new(config: JwtConfig) -> Result<Self, AuthError> {
        config.validate()?;
        let secret = config.secret.as_bytes();
        Ok(Self {
            config: config.clone(),
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        })
    }

    fn validation(&self) -> Validation {
        let mut validation = Validation::new(Algorithm::HS256);
        if let Some(iss) = &self.config.issuer {
            validation.set_issuer(&[iss.as_str()]);
        }
        validation
    }

    fn issue_token(&self, user_id: &Value, ttl_secs: u64, typ: &str) -> Result<String, AuthError> {
        let now = Utc::now();
        let claims = JwtClaims {
            sub: user_id.clone(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(ttl_secs as i64)).timestamp(),
            iss: self.config.issuer.clone(),
            typ: Some(typ.into()),
        };
        encode(&Header::default(), &claims, &self.encoding).map_err(AuthError::from)
    }

    /// Issue an access + refresh token pair for `user_id`.
    pub fn issue_tokens(&self, user_id: &Value) -> Result<TokenPair, AuthError> {
        Ok(TokenPair {
            access_token: self.issue_token(user_id, self.config.access_ttl, "access")?,
            refresh_token: self.issue_token(user_id, self.config.refresh_ttl, "refresh")?,
            token_type: "Bearer".into(),
            expires_in: self.config.access_ttl,
        })
    }

    /// Verify a token and return its claims.
    pub fn verify_token(&self, token: &str) -> Result<JwtClaims, AuthError> {
        decode::<JwtClaims>(token, &self.decoding, &self.validation())
            .map(|data| data.claims)
            .map_err(AuthError::from)
    }

    fn bearer_token(parts: &Parts) -> Option<String> {
        let value = parts.headers.get(header::AUTHORIZATION)?.to_str().ok()?;
        let token = value.strip_prefix("Bearer ")?.trim();
        if token.is_empty() {
            None
        } else {
            Some(token.to_string())
        }
    }
}

#[async_trait]
impl AuthStrategy for JwtStrategy {
    fn name(&self) -> &str {
        "jwt"
    }

    async fn authenticate(
        &self,
        parts: &Parts,
        _db: &DatabaseConnection,
    ) -> Result<Option<AuthIdentity>> {
        let token = match Self::bearer_token(parts) {
            Some(t) => t,
            None => return Ok(None),
        };
        let claims = self.verify_token(&token)?;
        Ok(Some(AuthIdentity {
            user_id: claims.sub,
        }))
    }
}
