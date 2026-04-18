use std::sync::Arc;
use crate::{engine::TemplateEngine, response::ViewResponse};
use doido_core::Result;

pub struct Renderer {
    engine: Arc<dyn TemplateEngine>,
    default_layout: String,
}

impl Renderer {
    pub fn new(engine: Arc<dyn TemplateEngine>, default_layout: impl Into<String>) -> Self {
        Self { engine, default_layout: default_layout.into() }
    }

    pub fn render(&self, response: &ViewResponse) -> Result<String> {
        let content = self.engine.render(&response.template, &response.context)?;

        let layout = match &response.layout {
            Some(l) if l.is_empty() => return Ok(content),
            Some(l) => l.clone(),
            None => self.default_layout.clone(),
        };

        if layout.is_empty() {
            return Ok(content);
        }

        let mut layout_ctx = response.context.clone();
        if let Some(obj) = layout_ctx.as_object_mut() {
            obj.insert(
                "content_for_layout".to_string(),
                serde_json::Value::String(content),
            );
        }
        self.engine.render(&format!("layouts/{}", layout), &layout_ctx)
    }
}
