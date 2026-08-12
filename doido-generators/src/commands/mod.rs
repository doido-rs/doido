pub mod console;
pub mod credentials;
pub mod db;
pub mod dbconsole;
pub mod destroy;
pub mod generate;
pub mod jobs;
pub mod new;
pub mod runner;
pub mod server;
pub mod worker;

use crate::generator::GeneratedFile;
use doido_core::Result;
use std::fs;
use std::path::Path;

pub(crate) fn write_files(files: &[GeneratedFile], root: &Path) -> Result<()> {
    for file in files {
        let dest = root.join(&file.path);
        if is_model_extension_path(&file.path) && dest.is_file() {
            doido_core::tracing::info!("skip {} (extension stub already exists)", file.path);
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, &file.content)?;
        doido_core::tracing::info!("create {}", file.path);
    }
    Ok(())
}

/// `app/models/<name>.rs` extension stubs are never overwritten; `_entities/` and
/// `mod.rs` are always rewritten.
fn is_model_extension_path(path: &str) -> bool {
    path.starts_with("app/models/")
        && !path.starts_with("app/models/_entities/")
        && path != "app/models/mod.rs"
        && path.ends_with(".rs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_files_creates_files_on_disk() {
        let dir = tempdir().unwrap();
        let files = vec![
            GeneratedFile {
                path: "src/main.rs".to_string(),
                content: "fn main() {}".to_string(),
            },
            GeneratedFile {
                path: "config/app.toml".to_string(),
                content: "[app]".to_string(),
            },
        ];
        write_files(&files, dir.path()).unwrap();
        assert!(dir.path().join("src/main.rs").exists());
        assert!(dir.path().join("config/app.toml").exists());
        assert_eq!(
            fs::read_to_string(dir.path().join("src/main.rs")).unwrap(),
            "fn main() {}"
        );
    }

    #[test]
    fn test_write_files_creates_nested_parent_directories() {
        let dir = tempdir().unwrap();
        let files = vec![GeneratedFile {
            path: "a/b/c/deep.rs".to_string(),
            content: "// deep".to_string(),
        }];
        write_files(&files, dir.path()).unwrap();
        assert!(dir.path().join("a/b/c/deep.rs").exists());
    }

    #[test]
    fn test_write_files_skips_existing_model_extension_stubs() {
        let dir = tempdir().unwrap();
        let models = dir.path().join("app/models");
        fs::create_dir_all(&models).unwrap();
        let existing = "// my custom validation\npub use super::_entities::post::*;\n";
        fs::write(models.join("post.rs"), existing).unwrap();

        let files = vec![GeneratedFile {
            path: "app/models/post.rs".to_string(),
            content: "pub use super::_entities::post::*;\n".to_string(),
        }];
        write_files(&files, dir.path()).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("app/models/post.rs")).unwrap(),
            existing
        );
    }

    #[test]
    fn test_write_files_always_overwrites_entities() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("app/models/_entities")).unwrap();
        fs::write(dir.path().join("app/models/_entities/post.rs"), "old").unwrap();

        let files = vec![GeneratedFile {
            path: "app/models/_entities/post.rs".to_string(),
            content: "new entity".to_string(),
        }];
        write_files(&files, dir.path()).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("app/models/_entities/post.rs")).unwrap(),
            "new entity"
        );
    }

    #[test]
    fn test_write_files_empty_content_creates_file() {
        let dir = tempdir().unwrap();
        let files = vec![GeneratedFile {
            path: "src/models/.gitkeep".to_string(),
            content: String::new(),
        }];
        write_files(&files, dir.path()).unwrap();
        assert!(dir.path().join("src/models/.gitkeep").exists());
    }
}
