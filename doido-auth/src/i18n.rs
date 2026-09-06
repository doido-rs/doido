//! Embedded auth locale files for the unified i18n catalog.

use doido_core::Result;

/// Registers built-in auth translations (`auth.*`) for supported locales.
pub fn register_locales() -> Result<()> {
    doido_core::i18n::register_yaml("en", include_str!("../locales/auth.en.yml"), Some("auth"))?;
    doido_core::i18n::register_yaml(
        "pt_BR",
        include_str!("../locales/auth.pt_BR.yml"),
        Some("auth"),
    )?;
    Ok(())
}
