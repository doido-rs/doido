//! Detect whether the current project depends on `doido-auth`.

use std::path::Path;

/// `true` when `Cargo.toml` at `path` lists `doido-auth` directly or enables the
/// `auth` feature on the `doido` meta crate.
pub fn project_has_doido_auth(path: impl AsRef<Path>) -> bool {
    let content = match std::fs::read_to_string(path.as_ref()) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let table: toml::Table = match content.parse() {
        Ok(t) => t,
        Err(_) => return false,
    };
    let deps = table
        .get("dependencies")
        .and_then(|d| d.as_table())
        .cloned()
        .unwrap_or_default();
    if deps.contains_key("doido-auth") {
        return true;
    }
    if let Some(doido) = deps.get("doido").and_then(|v| v.as_table()) {
        if (doido.contains_key("path") || doido.contains_key("version"))
            && feature_list(doido).iter().any(|f| f == "auth")
        {
            return true;
        }
    }
    false
}

fn feature_list(dep: &toml::Table) -> Vec<String> {
    dep.get("features")
        .and_then(|f| f.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detects_direct_doido_auth_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(
            &path,
            r#"
[dependencies]
doido-auth = "0.0.9"
"#,
        )
        .unwrap();
        assert!(project_has_doido_auth(&path));
    }

    #[test]
    fn detects_doido_meta_auth_feature() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(
            &path,
            r#"
[dependencies]
doido = { version = "0.0.9", features = ["auth", "sqlite"] }
"#,
        )
        .unwrap();
        assert!(project_has_doido_auth(&path));
    }

    #[test]
    fn absent_without_auth_dep() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
[dependencies]
doido = {{ version = "0.0.9", features = ["sqlite"] }}
"#
        )
        .unwrap();
        assert!(!project_has_doido_auth(&path));
    }
}
