//! Detect whether the current project depends on `doido-payments`.

use std::path::Path;

/// `true` when `Cargo.toml` at `path` lists `doido-payments`.
pub fn project_has_doido_payments(path: impl AsRef<Path>) -> bool {
    doido_payments::generators::project_has_doido_payments(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_direct_doido_payments_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(
            &path,
            r#"
[dependencies]
doido-payments = "0.0.1"
"#,
        )
        .unwrap();
        assert!(project_has_doido_payments(&path));
    }

    #[test]
    fn absent_without_payments_dep() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(
            &path,
            r#"
[dependencies]
doido = "0.0.9"
"#,
        )
        .unwrap();
        assert!(!project_has_doido_payments(&path));
    }
}
