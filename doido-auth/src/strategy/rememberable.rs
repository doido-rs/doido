//! `rememberable` strategy — resolves identity from the signed remember cookie.

pub use crate::middleware::rememberable::{
    cookie_value, forget, record_remember, RememberableMiddleware, REMEMBER_COOKIE,
};

use crate::identity::AuthIdentity;
use crate::strategy::AuthStrategy;
use async_trait::async_trait;
use doido_controller::CookieJar;
use doido_core::Result;
use doido_model::sea_orm::DatabaseConnection;
use http::header;
use http::request::Parts;

/// Auth strategy that resolves an identity from the signed remember cookie.
pub struct RememberStrategy;

#[async_trait]
impl AuthStrategy for RememberStrategy {
    fn name(&self) -> &str {
        "remember"
    }

    async fn authenticate(
        &self,
        parts: &Parts,
        _db: &DatabaseConnection,
    ) -> Result<Option<AuthIdentity>> {
        let header = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok());
        let jar = CookieJar::from_header(header, doido_controller::secret::key_base());
        match jar.get_signed(REMEMBER_COOKIE) {
            Some(raw) => {
                let user_id = serde_json::from_str::<serde_json::Value>(&raw)
                    .unwrap_or(serde_json::Value::String(raw));
                Ok(Some(AuthIdentity { user_id }))
            }
            None => Ok(None),
        }
    }
}
