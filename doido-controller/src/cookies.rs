//! A request/response cookie jar with plain and signed cookies (Rails
//! `cookies[...]` / `cookies.signed[...]`).
//!
//! Incoming cookies are parsed from the request `Cookie` header; outgoing
//! cookies are collected and rendered as `Set-Cookie` header values. Signed
//! cookies carry an HMAC-SHA256 signature so a tampered value is rejected on
//! read.

use crate::signing;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::collections::BTreeMap;

/// A cookie to be written back in a `Set-Cookie` header.
struct SetCookie {
    name: String,
    value: String,
}

/// Read incoming cookies and stage outgoing ones for a single request.
pub struct CookieJar {
    incoming: BTreeMap<String, String>,
    outgoing: Vec<SetCookie>,
    secret: Vec<u8>,
}

impl CookieJar {
    /// Build a jar from an optional `Cookie` header and a signing secret.
    pub fn from_header(cookie_header: Option<&str>, secret: Vec<u8>) -> Self {
        let mut incoming = BTreeMap::new();
        if let Some(header) = cookie_header {
            for (name, value) in header
                .split(';')
                .filter_map(|pair| pair.trim().split_once('='))
            {
                incoming.insert(name.to_string(), value.to_string());
            }
        }
        Self {
            incoming,
            outgoing: Vec::new(),
            secret,
        }
    }

    /// Read a plain incoming cookie value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.incoming.get(name).map(String::as_str)
    }

    /// Stage a plain cookie to be written on the response.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.outgoing.push(SetCookie {
            name: name.into(),
            value: value.into(),
        });
    }

    /// Read a signed incoming cookie, returning `None` if it is missing,
    /// malformed, or its signature does not verify.
    pub fn get_signed(&self, name: &str) -> Option<String> {
        let raw = self.incoming.get(name)?;
        let (msg, sig) = raw.split_once('.')?;
        if !signing::verify(&self.secret, msg.as_bytes(), sig) {
            return None;
        }
        let bytes = URL_SAFE_NO_PAD.decode(msg).ok()?;
        String::from_utf8(bytes).ok()
    }

    /// Stage a signed cookie (`base64url(value).signature`).
    pub fn set_signed(&mut self, name: impl Into<String>, value: impl AsRef<str>) {
        let msg = URL_SAFE_NO_PAD.encode(value.as_ref().as_bytes());
        let sig = signing::sign(&self.secret, msg.as_bytes());
        self.outgoing.push(SetCookie {
            name: name.into(),
            value: format!("{msg}.{sig}"),
        });
    }

    /// Render the staged cookies as `Set-Cookie` header values.
    pub fn to_set_cookie_headers(&self) -> Vec<String> {
        self.outgoing
            .iter()
            .map(|c| format!("{}={}; Path=/; HttpOnly", c.name, c.value))
            .collect()
    }
}
