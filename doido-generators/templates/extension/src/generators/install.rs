//! `{doido_ext_snake}:install` — starter generator for consuming apps.

use super::{ExtensionGenerator, GeneratedFile};
use doido_core::Result;

pub struct InstallGenerator;

impl ExtensionGenerator for InstallGenerator {
    fn name(&self) -> &str {
        "{doido_ext_snake}:install"
    }

    fn generate(&self, _args: &[&str]) -> Result<Vec<GeneratedFile>> {
        Ok(vec![GeneratedFile {
            path: "README.{doido_ext_snake}.md".to_string(),
            content: template::README.to_string(),
        }])
    }
}

mod template {
    pub const README: &str = include_str!("../../../templates/install/README.md.template");
}
