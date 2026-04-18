pub trait TemplateEngine: Send + Sync {
    fn render(&self, template: &str, context: &serde_json::Value) -> doido_core::Result<String>;
    fn reload(&self) -> doido_core::Result<()>;
}
