//! Compile-time detection of "API-only" projects.
//!
//! A project generated with `doido new --api` carries an `api_only = true` marker
//! under the `[app]` table of `config/application.toml`. The route macros read it
//! at expansion time (via `CARGO_MANIFEST_DIR`, which points at the crate being
//! compiled — i.e. the application) so that `resources!` can omit the HTML-form
//! routes (`new`/`edit`) that make no sense for a JSON API.
//!
//! Reads are best-effort: a missing/unreadable file means "not an API" so a plain
//! project keeps every REST route. `application.toml` is not loaded at runtime;
//! this is purely a build-time signal.

use std::path::{Path, PathBuf};

/// Path to the app's `config/application.toml`, if it exists. Returned so the
/// generated code can `include_bytes!` it and let rustc re-run the macro when the
/// marker changes (stable proc-macros don't track `fs::read` on their own).
pub(crate) fn manifest_config_path() -> Option<PathBuf> {
    let dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let path = Path::new(&dir).join("config").join("application.toml");
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

/// Whether the project being compiled is API-only (`[app] api_only = true`).
pub(crate) fn api_only() -> bool {
    manifest_config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|contents| has_api_only(&contents))
        .unwrap_or(false)
}

/// Scan a flat `application.toml` for an `api_only = true` assignment. Comments
/// (`#`) are stripped; the value is matched case-insensitively for `true`.
fn has_api_only(contents: &str) -> bool {
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "api_only" && value.trim().eq_ignore_ascii_case("true") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::has_api_only;

    #[test]
    fn detects_api_only_true() {
        assert!(has_api_only("[app]\nname = \"x\"\napi_only = true\n"));
        assert!(has_api_only("api_only=true"));
        assert!(has_api_only("api_only = TRUE  # marker"));
    }

    #[test]
    fn ignores_absent_or_false() {
        assert!(!has_api_only("[app]\nname = \"x\"\n"));
        assert!(!has_api_only("api_only = false"));
        assert!(!has_api_only("# api_only = true"));
        assert!(!has_api_only("not_api_only = true"));
    }
}
