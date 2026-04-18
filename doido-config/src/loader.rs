use std::path::Path;
use doido_core::{Result, anyhow::Context as _};

pub fn load_toml(path: &Path) -> Result<Option<toml::Value>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let value: toml::Value = toml::from_str(&content)
        .with_context(|| format!("failed to parse TOML at {}", path.display()))?;
    Ok(Some(value))
}

pub fn deep_merge(base: toml::Value, over: toml::Value) -> toml::Value {
    match (base, over) {
        (toml::Value::Table(mut base_map), toml::Value::Table(over_map)) => {
            for (k, v) in over_map {
                let entry = base_map
                    .entry(k)
                    .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
                *entry = deep_merge(entry.clone(), v);
            }
            toml::Value::Table(base_map)
        }
        (_, over) => over,
    }
}

/// Load and merge TOML layers: base config, then env-specific override.
/// Credentials layer is added in Task 6.
pub fn load_layers(root: &Path, env: &str) -> Result<toml::Value> {
    // 1. Base config — required
    let base_path = root.join("config/doido.toml");
    let mut merged = load_toml(&base_path)?
        .ok_or_else(|| doido_core::anyhow::anyhow!(
            "config/doido.toml not found in {}",
            root.display()
        ))?;

    // 2. Environment-specific override — optional
    let env_path = root.join(format!("config/doido.{env}.toml"));
    if let Some(env_value) = load_toml(&env_path)? {
        merged = deep_merge(merged, env_value);
    }

    // 3. Encrypted credentials — optional file, but key is required when file exists
    let cred_path = root.join("config/credentials.toml.enc");
    if cred_path.exists() {
        let key = crate::crypto::load_master_key(root)
            .context("failed to load master key for credentials.toml.enc")?;
        let encoded = std::fs::read_to_string(&cred_path)
            .context("failed to read config/credentials.toml.enc")?;
        let plaintext = crate::crypto::decrypt_credentials(&encoded, &key)
            .context("failed to decrypt config/credentials.toml.enc")?;
        let cred_value: toml::Value = toml::from_str(&plaintext)
            .context("failed to parse decrypted credentials as TOML")?;
        merged = deep_merge(merged, cred_value);
    }

    Ok(merged)
}
