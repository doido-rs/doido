//! Runtime environment selection, driven by the `DOIDO_ENV` variable.

/// The application environment. Selects which `config/<env>.yml` file is read.
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
