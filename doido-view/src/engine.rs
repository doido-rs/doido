pub trait TemplateEngine: Send + Sync {
    fn render(&self, template: &str, context: &serde_json::Value) -> doido_core::Result<String>;
    fn reload(&self) -> doido_core::Result<()>;

    /// Render a template by its **exact** registered name (including any
    /// `.html.tera` / `.text.tera` suffix), bypassing the `.html.tera` suffix
    /// that [`render`](Self::render) appends. Used e.g. to render a mailer's
    /// text part. The default delegates to [`render`](Self::render) for engines
    /// that do not distinguish variants.
    fn render_named(&self, name: &str, context: &serde_json::Value) -> doido_core::Result<String> {
        self.render(name, context)
    }
}
