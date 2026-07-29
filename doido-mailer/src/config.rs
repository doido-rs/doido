//! Config-driven deliverer selection (Action Mailer `delivery_method`), read
//! from the `mailer` section of `config/<env>.yml`. Mirrors the `config` modules
//! in `doido-jobs`/`doido-cache`: callers only ever see an `Arc<dyn Deliverer>`.
//!
//! ```yaml
//! mailer:
//!   type: smtp              # log (default) | test | smtp | sendmail
//!   smtp:
//!     address: smtp.example.com:587
//!     starttls: true
//!     username: apikey
//!     password: s3cret
//! ```

use crate::deliverer::{Deliverer, LogDeliverer, TestDeliverer};
use crate::sendmail::SendmailDeliverer;
use crate::smtp::SmtpDeliverer;
use doido_core::Environment;
use serde::Deserialize;
use std::sync::Arc;

/// Which deliverer to use. YAML key is `type`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    /// Log the mail instead of sending it (safe default for dev/test).
    #[default]
    Log,
    /// Capture in memory (a fresh [`TestDeliverer`]).
    Test,
    /// Talk SMTP to `smtp.address`, optionally STARTTLS + AUTH.
    Smtp,
    /// Pipe to the local `sendmail` binary.
    Sendmail,
}

/// SMTP connection settings (`mailer.smtp`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SmtpSettings {
    /// `host:port` of the SMTP server.
    pub address: Option<String>,
    #[serde(default)]
    pub starttls: bool,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Permit AUTH over a plain (non-TLS) connection. Off by default.
    #[serde(default)]
    pub allow_insecure_auth: bool,
}

/// Runtime mailer configuration.
#[derive(Debug, Clone, Default)]
pub struct MailerConfig {
    pub backend: Backend,
    pub smtp: SmtpSettings,
}

impl MailerConfig {
    /// Build the configured deliverer behind an `Arc<dyn Deliverer>`.
    pub fn build(&self) -> Arc<dyn Deliverer> {
        match self.backend {
            Backend::Log => Arc::new(LogDeliverer),
            Backend::Test => Arc::new(TestDeliverer::new()),
            Backend::Sendmail => Arc::new(SendmailDeliverer::default()),
            Backend::Smtp => {
                let addr = self
                    .smtp
                    .address
                    .clone()
                    .unwrap_or_else(|| "localhost:25".to_string());
                let mut deliverer = SmtpDeliverer::new(addr);
                if self.smtp.starttls {
                    deliverer = deliverer.starttls();
                }
                if let (Some(user), Some(pass)) = (&self.smtp.username, &self.smtp.password) {
                    deliverer = deliverer.credentials(user, pass);
                    if self.smtp.allow_insecure_auth {
                        deliverer = deliverer.allow_insecure_auth();
                    }
                }
                Arc::new(deliverer)
            }
        }
    }
}

/// Parsed `mailer` section, before defaults are applied.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MailerSettings {
    #[serde(default, rename = "type")]
    pub backend: Backend,
    #[serde(default)]
    pub smtp: SmtpSettings,
}

impl MailerSettings {
    pub fn into_config(self) -> MailerConfig {
        MailerConfig {
            backend: self.backend,
            smtp: self.smtp,
        }
    }
}

/// File config deserialized from `config/<env>.yml`; only `mailer` is read.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MailerFileConfig {
    #[serde(default)]
    pub mailer: MailerSettings,
}

impl MailerFileConfig {
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

/// Load the current environment's [`MailerConfig`], falling back to the default
/// (log deliverer) when the file is missing or has no `mailer` section.
pub fn load() -> MailerConfig {
    MailerFileConfig::load()
        .map(|c| c.mailer.into_config())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_smtp_section() {
        let yaml = "mailer:\n  type: smtp\n  smtp:\n    address: mail.x:587\n    \
                    starttls: true\n    username: u\n    password: p\n";
        let cfg = MailerFileConfig::from_yaml(yaml)
            .unwrap()
            .mailer
            .into_config();
        assert_eq!(cfg.backend, Backend::Smtp);
        assert_eq!(cfg.smtp.address.as_deref(), Some("mail.x:587"));
        assert!(cfg.smtp.starttls);
        assert_eq!(cfg.smtp.username.as_deref(), Some("u"));
    }

    #[test]
    fn absent_section_defaults_to_log() {
        let cfg = MailerFileConfig::from_yaml("server:\n  port: 3000\n")
            .unwrap()
            .mailer
            .into_config();
        assert_eq!(cfg.backend, Backend::Log);
    }
}
