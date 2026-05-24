pub mod console;
pub mod credentials;
pub mod db;
pub mod generate;
pub mod jobs;
pub mod new;
pub mod server;
pub mod worker;

use doido_core::Result;
use doido_generators::GeneratedFile;
use std::fs;
use std::path::Path;

pub(crate) fn write_files(files: &[GeneratedFile], root: &Path) -> Result<()> {
    for file in files {
        let dest = root.join(&file.path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, &file.content)?;
        println!("  create  {}", file.path);
    }
    Ok(())
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
