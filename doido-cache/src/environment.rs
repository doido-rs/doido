//! Runtime environment selection, driven by the `DOIDO_ENV` variable.
//!
//! Mirrors `doido-controller`'s environment so the cache layer can resolve the
//! same `config/<env>.yml` file without depending on the controller crate.

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
}
