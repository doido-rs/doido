//! Per-environment application configuration loaded from `config/<env>.yml`.
//!
//! [`Config`] is a trait so applications can supply their own backing store;
//! [`YamlConfig`] is the default implementation that deserializes the YAML file
//! for the environment reported by [`Environment::get_env`].

use crate::environment::Environment;
use serde::Deserialize;

/// Re-exported so `config::LoggerConfig` resolves; the logger config lives in
/// `doido-core` alongside the logger it drives.
pub use doido_core::logger::LoggerConfig;

/// Server bind settings. The listen address is the `bind` IP joined with `port`
/// (e.g. `0.0.0.0:3000`).
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

/// Application configuration. Used as a trait object (`Box<dyn Config>`) so the
/// backing store can be swapped without touching call sites.
pub trait Config: Send + Sync {
    /// Server bind/port settings.
    fn server(&self) -> &ServerConfig;
    /// Logging settings.
    fn logger(&self) -> &LoggerConfig;
}

/// File-based [`Config`] deserialized from `config/<env>.yml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct YamlConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub logger: LoggerConfig,
}

impl Config for YamlConfig {
    fn server(&self) -> &ServerConfig {
        &self.server
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
    fn parses_logger_level() {
        let config = YamlConfig::from_yaml("logger:\n  level: debug\n").unwrap();
        assert_eq!(config.logger().level, "debug");
        assert_eq!(
            config.logger().directives(),
            doido_core::logger::directives_for_level("debug")
        );
    }

    #[test]
    fn explicit_directives_override_level() {
        let yaml = "logger:\n  level: info\n  directives: warn,my_app=debug\n";
        let config = YamlConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.logger().directives(), "warn,my_app=debug");
    }

    #[test]
    fn defaults_to_info_when_logger_section_absent() {
        let config = YamlConfig::from_yaml("server:\n  bind: 0.0.0.0\n  port: 3000\n").unwrap();
        assert_eq!(config.logger().level, "info");
        assert!(config.logger().sql);
        assert!(config.logger().file.is_none());
        assert_eq!(
            config.logger().directives(),
            doido_core::logger::DEFAULT_DIRECTIVES
        );
    }
}
