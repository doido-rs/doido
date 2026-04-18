pub mod types;
pub mod loader;
pub mod env_override;
pub mod crypto;

pub use types::{Config, DatabaseConfig, LogConfig, ServerConfig, ViewConfig};

use std::{path::Path, sync::Arc};
use doido_core::Result;

impl Config {
    /// Load config using the current directory as root, environment from `DOIDO_ENV`
    /// (defaults to `"development"`).
    pub fn load() -> Result<Arc<Self>> {
        let env = std::env::var("DOIDO_ENV").unwrap_or_else(|_| "development".to_string());
        Self::load_from_env(Path::new("."), &env)
    }

    /// Load config from `root`, environment from `DOIDO_ENV`.
    pub fn load_from(root: impl AsRef<Path>) -> Result<Arc<Self>> {
        let env = std::env::var("DOIDO_ENV").unwrap_or_else(|_| "development".to_string());
        Self::load_from_env(root.as_ref(), &env)
    }

    /// Load config from `root` with an explicit environment name.
    /// Useful in tests to avoid depending on the `DOIDO_ENV` env var.
    ///
    /// Loading order (each layer overrides the previous):
    /// 1. `config/doido.toml`             — base config
    /// 2. `config/doido.<env>.toml`       — environment override
    /// 3. `config/credentials.toml.enc`   — encrypted secrets (if present)
    /// 4. Env vars with `SECTION__KEY` notation
    pub fn load_from_env(root: &Path, env: &str) -> Result<Arc<Self>> {
        let mut merged = loader::load_layers(root, env)?;
        env_override::apply_env_overrides(&mut merged);
        let toml_str = toml::to_string(&merged)
            .map_err(|e| doido_core::anyhow::anyhow!("cannot serialize merged config: {e}"))?;
        let config: Self = toml::from_str(&toml_str)
            .map_err(|e| doido_core::anyhow::anyhow!("cannot deserialize config: {e}"))?;
        Ok(Arc::new(config))
    }
}
