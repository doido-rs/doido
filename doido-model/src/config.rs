//! Per-environment database configuration loaded from `config/<env>.yml`.
//!
//! Mirrors `doido-controller`'s config: [`Config`] is a trait so applications
//! can supply their own backing store, and [`YamlConfig`] is the default
//! implementation that deserializes the `database` section of the YAML file for
//! the environment reported by [`Environment::get_env`].

use crate::environment::Environment;
use serde::Deserialize;

/// Re-exported so `config::LoggerConfig` resolves; the logger config lives in
/// `doido-core` alongside the logger it drives. The model layer reads it for the
/// `sql` toggle (whether sea-orm logs each statement).
pub use doido_core::logger::LoggerConfig;

/// Database connection settings.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Connection URL, e.g. `postgres://localhost/my_app_development` or
    /// `sqlite://db/development.db`.
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://db/development.db".to_string(),
        }
    }
}

/// Model-layer configuration. Used as a trait object (`Box<dyn Config>`) so the
/// backing store can be swapped without touching call sites.
pub trait Config: Send + Sync {
    /// Database connection settings.
    fn database(&self) -> &DatabaseConfig;
    /// Logging settings; the model layer reads the `sql` toggle.
    fn logger(&self) -> &LoggerConfig;
}

/// File-based [`Config`] deserialized from the `database` and `logger` sections
/// of `config/<env>.yml`. Other sections (e.g. `server`) are ignored.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct YamlConfig {
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub logger: LoggerConfig,
}

impl Config for YamlConfig {
    fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    fn logger(&self) -> &LoggerConfig {
        &self.logger
    }
}

impl YamlConfig {
    /// Loads `config/<env>.yml` for the environment from [`Environment::get_env`].
    pub fn load() -> std::io::Result<Self> {
        Self::load_env(Environment::get_env())
    }

    /// Loads `config/<env>.yml` for a specific environment.
    pub fn load_env(env: Environment) -> std::io::Result<Self> {
        let path = format!("config/{}.yml", env.as_str());
        let contents = std::fs::read_to_string(&path)?;
        Self::from_yaml(&contents)
    }

    /// Parses a [`YamlConfig`] from a YAML string.
    pub fn from_yaml(yaml: &str) -> std::io::Result<Self> {
        serde_norway::from_str(yaml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Loads the current environment's configuration as a trait object, falling
/// back to [`Default`] values when the file is missing or invalid.
pub fn load() -> Box<dyn Config> {
    Box::new(YamlConfig::load().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_database_url_and_ignores_other_sections() {
        let yaml = "server:\n  bind: 0.0.0.0\n  port: 3000\ndatabase:\n  url: postgres://localhost/app_development\n";
        let config = YamlConfig::from_yaml(yaml).unwrap();
        assert_eq!(
            config.database().url,
            "postgres://localhost/app_development"
        );
    }

    #[test]
    fn defaults_when_database_section_absent() {
        let config = YamlConfig::from_yaml("server:\n  port: 3000\n").unwrap();
        assert_eq!(config.database().url, "sqlite://db/development.db");
    }
}
