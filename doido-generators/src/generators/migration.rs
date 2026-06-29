use crate::generator::{GeneratedFile, Generator};
use crate::generators::to_snake;
use chrono::Utc;
use doido_core::Result;

pub struct MigrationGenerator;

impl Generator for MigrationGenerator {
    fn name(&self) -> &str {
        "migration"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("migration generator requires a name argument")
        })?;
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let snake = to_snake(name);
        Ok(vec![GeneratedFile {
            path: format!("db/migrations/{}_{}.rs", timestamp, snake),
            content: crate::templates::get("migration/migration.rs.template"),
        }])
    }
}
