//! `doido destroy <generator> <name>` — undo a generator by removing the files
//! it created. Shared/injected files (`mod.rs`/`routes.rs`/`lib.rs`) are left in
//! place because they hold other registrations.

use crate::GeneratedFile;
use std::path::Path;

/// Whether `path` is a shared/injected file that destroy must not delete.
fn is_shared(path: &str) -> bool {
    path.ends_with("mod.rs") || path.ends_with("routes.rs") || path.ends_with("lib.rs")
}

/// The paths destroy would remove for `files`: the standalone generated files.
pub fn destroyable_paths(files: &[GeneratedFile]) -> Vec<String> {
    files
        .iter()
        .filter(|f| !is_shared(&f.path))
        .map(|f| f.path.clone())
        .collect()
}

/// Run the named generator to learn its files, then delete the destroyable ones
/// under the current directory. Returns the paths actually removed.
pub fn run(generator: &str, args: &[&str]) -> doido_core::Result<Vec<String>> {
    let files = crate::default_registry().run(generator, args)?;
    let mut removed = Vec::new();
    for path in destroyable_paths(&files) {
        if Path::new(&path).exists() {
            std::fs::remove_file(&path)
                .map_err(|e| doido_core::anyhow::anyhow!("failed to remove {path}: {e}"))?;
            removed.push(path);
        }
    }
    Ok(removed)
}
