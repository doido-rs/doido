//! `doido generate storage:install` — the `active_storage:install` analogue.
//!
//! Emits a migration creating the `storage_blobs`, `storage_attachments` and
//! `storage_variant_records` tables (via the `doido_model::migration` builders),
//! registers it in `db/migration/src/lib.rs`, and appends a `storage:` section to
//! `config/development.yml` and `config/test.yml` when those files exist.

use crate::generator::{GeneratedFile, Generator};
use crate::generators::bootstrap_migrations::{
    apply_bootstrap_migrations, storage_config_section, storage_migration_installed,
};
use crate::generators::migration_support::{MIGRATION_LIB_BASE, MIGRATION_SRC_DIR};
use doido_core::Result;

pub struct StorageInstallGenerator;

/// Append the storage section to `path` if the file exists and has none yet.
fn config_file(path: &str, active: &str) -> Option<GeneratedFile> {
    let existing = std::fs::read_to_string(path).ok()?;
    if existing.contains("storage:") {
        return None;
    }
    Some(GeneratedFile {
        path: path.to_string(),
        content: format!(
            "{}{}",
            existing.trim_end(),
            format!("\n{}", storage_config_section(active))
        ),
    })
}

impl Generator for StorageInstallGenerator {
    fn name(&self) -> &str {
        "storage:install"
    }

    fn generate(&self, _args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let lib_path = format!("{MIGRATION_SRC_DIR}/lib.rs");
        let existing =
            std::fs::read_to_string(&lib_path).unwrap_or_else(|_| MIGRATION_LIB_BASE.to_string());

        let mut files = Vec::new();
        if storage_migration_installed(&existing) {
            files.push(GeneratedFile {
                path: lib_path,
                content: existing,
            });
        } else {
            let (lib, migrations) = apply_bootstrap_migrations(&existing, false);
            files.push(GeneratedFile {
                path: lib_path,
                content: lib,
            });
            for (module, content) in migrations {
                files.push(GeneratedFile {
                    path: format!("{MIGRATION_SRC_DIR}/{module}.rs"),
                    content,
                });
            }
        }

        // Best-effort: wire the `storage` config section into existing env files.
        if let Some(f) = config_file("config/development.yml", "local") {
            files.push(f);
        }
        if let Some(f) = config_file("config/test.yml", "test") {
            files.push(f);
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::bootstrap_migrations::STORAGE_MIGRATION_MODULE;

    #[test]
    fn emits_migration_and_registers_it() {
        let files = StorageInstallGenerator.generate(&[]).unwrap();
        // A migration file plus the updated lib.rs (config files depend on cwd).
        let migration = files
            .iter()
            .find(|f| f.path.contains(STORAGE_MIGRATION_MODULE) && f.path.ends_with(".rs"))
            .expect("migration file emitted");
        assert!(migration.content.contains("storage_blobs"));
        assert!(migration.content.contains("storage_attachments"));
        assert!(migration.content.contains("storage_variant_records"));
        assert!(migration
            .content
            .contains("impl MigrationName for Migration"));
        assert!(!migration.content.contains("DeriveMigrationName"));
        let module = migration
            .path
            .strip_prefix("db/migration/src/")
            .unwrap()
            .strip_suffix(".rs")
            .unwrap();
        assert!(migration.content.contains(&format!("\"{module}\"")));

        let lib = files
            .iter()
            .find(|f| f.path.ends_with("lib.rs"))
            .expect("lib.rs emitted");
        assert!(lib.content.contains(STORAGE_MIGRATION_MODULE));
    }
}
