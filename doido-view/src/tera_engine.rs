use crate::engine::TemplateEngine;
use doido_core::{anyhow::Context as _, Result};
use std::sync::RwLock;

pub struct TeraEngine {
    tera: RwLock<tera::Tera>,
    templates_dir: String,
}

impl TeraEngine {
    pub fn new(templates_dir: &str) -> Result<Self> {
        let pattern = format!("{}/**/*.tera", templates_dir);
        let tera = tera::Tera::new(&pattern)
            .with_context(|| format!("failed to load templates from {templates_dir}"))?;
        Ok(Self {
            tera: RwLock::new(tera),
            templates_dir: templates_dir.to_string(),
        })
    }
}

impl TemplateEngine for TeraEngine {
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String> {
        let template_name = format!("{}.html.tera", template);
        let ctx = tera::Context::from_value(context.clone())
            .map_err(|e| doido_core::anyhow::anyhow!("invalid template context: {e}"))?;
        self.tera
            .read()
            .unwrap()
            .render(&template_name, &ctx)
            .map_err(|e| doido_core::anyhow::anyhow!("template '{}' render failed: {e}", template))
    }

    fn reload(&self) -> Result<()> {
        let pattern = format!("{}/**/*.tera", self.templates_dir);
        let tera = tera::Tera::new(&pattern)
            .with_context(|| format!("reload failed for {}", self.templates_dir))?;
        *self.tera.write().unwrap() = tera;
        Ok(())
    }
}
