//! Thin app boot helpers — call [`prepare`] from `main` before [`doido::Doido::run`].

/// Loads `.env` in development when present (via `dotenvy`). Production and test
/// rely on the process environment or your orchestrator (Compose, Dokku, CI).
pub fn prepare() {
    let env = std::env::var("DOIDO_ENV").unwrap_or_else(|_| "development".into());
    if env == "development" {
        let _ = dotenvy::dotenv();
    }
}
