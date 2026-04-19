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
            content: "use sea_orm_migration::prelude::*;\n\n#[derive(DeriveMigrationName)]\npub struct Migration;\n\n#[async_trait::async_trait]\nimpl MigrationTrait for Migration {\n    async fn up(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {\n        // TODO: implement migration\n        Ok(())\n    }\n\n    async fn down(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {\n        // TODO: implement rollback\n        Ok(())\n    }\n}\n".to_string(),
        }])
    }
}
