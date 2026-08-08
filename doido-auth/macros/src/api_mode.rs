use std::path::{Path, PathBuf};

pub(crate) fn api_only() -> bool {
    manifest_config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|contents| has_api_only(&contents))
        .unwrap_or(false)
}

fn manifest_config_path() -> Option<PathBuf> {
    let dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let path = Path::new(&dir).join("config").join("application.toml");
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

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
