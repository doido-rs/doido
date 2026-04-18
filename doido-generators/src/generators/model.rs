use crate::generator::{GeneratedFile, Generator};
use crate::generators::to_snake;
use doido_core::Result;

pub struct ModelGenerator;

impl Generator for ModelGenerator {
    fn name(&self) -> &str { "model" }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied()
            .ok_or_else(|| doido_core::anyhow::anyhow!("model generator requires a name argument"))?;
        let snake = to_snake(name);
        Ok(vec![GeneratedFile {
            path: format!("app/models/{}.rs", snake),
            content: format!(
                "use doido_model::sea_orm::entity::prelude::*;\n\n#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]\n#[sea_orm(table_name = \"{snake}s\")]\npub struct Model {{\n    #[sea_orm(primary_key)]\n    pub id: i32,\n}}\n\n#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\npub enum Relation {{}}\n\nimpl ActiveModelBehavior for ActiveModel {{}}\n",
            ),
        }])
    }
}
