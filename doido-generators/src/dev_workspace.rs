//! Runtime detection of whether the running `doido` / `doido-generators` binary
//! lives inside a local Doido framework checkout (path deps) or is an isolated /
//! published install (crates.io version deps matching this binary's release).

use std::path::{Path, PathBuf};

/// How generated apps should depend on first-party `doido-*` crates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyMode {
    /// Use `path = "…"` deps against the discovered workspace root.
    pub use_path: bool,
    /// Absolute workspace root when [`Self::use_path`] is true.
    pub workspace_path: String,
    /// Crates.io version pinned when [`Self::use_path`] is false.
    pub version: &'static str,
}

impl DependencyMode {
    /// Resolve dependency style from the running binary's location on disk.
    pub fn resolve() -> Self {
        let version = env!("CARGO_PKG_VERSION");
        if let Some(root) = find_dev_workspace_root() {
            Self {
                use_path: true,
                workspace_path: root.display().to_string(),
                version,
            }
        } else {
            Self {
                use_path: false,
                workspace_path: String::new(),
                version,
            }
        }
    }
}

/// Returns the workspace root when `path` looks like a Doido framework checkout.
pub fn is_doido_workspace(path: &Path) -> bool {
    path.join("doido").join("Cargo.toml").is_file()
        && path.join("doido-generators").join("Cargo.toml").is_file()
        && path.join("doido-core").join("Cargo.toml").is_file()
}

/// Walks upward from the running executable until a Doido workspace is found.
pub fn find_dev_workspace_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    for ancestor in exe.ancestors() {
        if is_doido_workspace(ancestor) {
            return ancestor
                .canonicalize()
                .ok()
                .or_else(|| Some(ancestor.to_path_buf()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn is_doido_workspace_requires_core_crates() {
        let dir = tempdir().unwrap();
        assert!(!is_doido_workspace(dir.path()));

        fs::create_dir_all(dir.path().join("doido")).unwrap();
        fs::write(
            dir.path().join("doido/Cargo.toml"),
            "[package]\nname = \"doido\"\n",
        )
        .unwrap();
        assert!(!is_doido_workspace(dir.path()));

        fs::create_dir_all(dir.path().join("doido-generators")).unwrap();
        fs::write(
            dir.path().join("doido-generators/Cargo.toml"),
            "[package]\nname = \"doido-generators\"\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("doido-core")).unwrap();
        fs::write(
            dir.path().join("doido-core/Cargo.toml"),
            "[package]\nname = \"doido-core\"\n",
        )
        .unwrap();
        assert!(is_doido_workspace(dir.path()));
    }

    #[test]
    fn resolve_uses_path_when_test_binary_runs_inside_checkout() {
        let mode = DependencyMode::resolve();
        // Integration/unit tests execute from `<workspace>/target/debug/deps/…`.
        if find_dev_workspace_root().is_some() {
            assert!(mode.use_path);
            assert!(mode.workspace_path.contains("doido"));
        } else {
            assert!(!mode.use_path);
        }
    }
}
