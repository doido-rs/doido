//! Runtime environment selection, driven by `DOIDO_ENV`.
//!
//! Mirrors `doido-cache`'s environment so the storage layer resolves the same
//! `config/<env>.yml` file without depending on the controller crate.

/// The application environment. Selects which `config/<env>.yml` file is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Environment {
    /// Reads the current environment from `DOIDO_ENV` (`development` default).
    pub fn get_env() -> Environment {
        match std::env::var("DOIDO_ENV").as_deref() {
            Ok("production") => Environment::Production,
            Ok("test") => Environment::Test,
            _ => Environment::Development,
        }
    }

    /// Lowercase name used for the `config/<env>.yml` file and for display.
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Test => "test",
            Environment::Production => "production",
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
