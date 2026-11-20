//! Runtime environment selection, driven by the `DOIDO_ENV` variable.
//!
//! The single, canonical [`Environment`] for the whole framework. Every crate
//! that needs to resolve `config/<env>.yml` uses `doido_core::Environment`.

use std::path::{Path, PathBuf};

/// The application environment. Selects which `config/environments/<env>.yml` file is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Environment {
    /// Reads the current environment from `DOIDO_ENV`.
    ///
    /// Recognized values are `development`, `test`, and `production`. An unset
    /// or unrecognized value falls back to [`Environment::Development`].
    pub fn get_env() -> Environment {
        match std::env::var("DOIDO_ENV").as_deref() {
            Ok("production") => Environment::Production,
            Ok("test") => Environment::Test,
            _ => Environment::Development,
        }
    }

    /// Lowercase name used for the environment config file and for display.
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Test => "test",
            Environment::Production => "production",
        }
    }

    /// Path to the per-environment configuration file relative to `root`.
    pub fn config_path_in<P: AsRef<Path>>(&self, root: P) -> PathBuf {
        root.as_ref()
            .join("config")
            .join("environments")
            .join(format!("{}.yml", self.as_str()))
    }

    /// Path to the per-environment configuration file relative to the current working directory.
    pub fn config_path(&self) -> PathBuf {
        self.config_path_in("")
    }

    /// Returns `true` if this is [`Environment::Development`].
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }

    /// Returns `true` if this is [`Environment::Test`].
    pub fn is_test(&self) -> bool {
        matches!(self, Environment::Test)
    }

    /// Returns `true` if this is [`Environment::Production`].
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_maps_each_variant() {
        assert_eq!(Environment::Development.as_str(), "development");
        assert_eq!(Environment::Test.as_str(), "test");
        assert_eq!(Environment::Production.as_str(), "production");
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(Environment::Production.to_string(), "production");
    }

    #[test]
    fn predicates_identify_environment() {
        assert!(Environment::Development.is_development());
        assert!(!Environment::Development.is_test());
        assert!(!Environment::Development.is_production());

        assert!(Environment::Test.is_test());
        assert!(!Environment::Test.is_development());
        assert!(!Environment::Test.is_production());

        assert!(Environment::Production.is_production());
        assert!(!Environment::Production.is_development());
        assert!(!Environment::Production.is_test());
    }

    #[test]
    fn config_path_strictly_targets_environments_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Even if legacy config/development.yml exists, environments/ is strictly returned
        let legacy_file = dir.path().join("config/development.yml");
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(&legacy_file, "server:\n  port: 4000\n").unwrap();

        assert_eq!(
            Environment::Development.config_path_in(dir.path()),
            dir.path().join("config/environments/development.yml")
        );
    }
}
