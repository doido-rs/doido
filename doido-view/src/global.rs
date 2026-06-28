//! Process-global template engine, installed once at boot and reached from
//! request handlers via `Context::render`.
//!
//! Mirrors the framework's other boot-time singletons (the DB pool, inflections):
//! the application installs an engine with [`init`]/[`set_engine`] and controllers
//! render through [`render`] without threading the engine through every call.

use crate::engine::TemplateEngine;
use crate::tera_engine::TeraEngine;
use doido_core::Result;
use std::sync::{Arc, OnceLock};

static ENGINE: OnceLock<Arc<dyn TemplateEngine>> = OnceLock::new();

/// Installs a template engine globally. Idempotent: a second call is ignored.
pub fn set_engine(engine: Arc<dyn TemplateEngine>) {
    let _ = ENGINE.set(engine);
}

/// Installs the default [`TeraEngine`] over `templates_dir` (e.g. `app/views`),
/// loading every `**/*.tera` file under it. Idempotent. Call once at boot.
pub fn init(templates_dir: &str) -> Result<()> {
    if ENGINE.get().is_some() {
        return Ok(());
    }
    let engine = TeraEngine::new(templates_dir)?;
    set_engine(Arc::new(engine));
    Ok(())
}

/// Returns the installed engine, if any.
pub fn try_engine() -> Option<Arc<dyn TemplateEngine>> {
    ENGINE.get().cloned()
}

/// Renders `template` (without the `.html.tera` suffix) with `context` to an
/// HTML string. Errors if no engine was installed or the template fails.
pub fn render(template: &str, context: &serde_json::Value) -> Result<String> {
    let engine = ENGINE.get().ok_or_else(|| {
        doido_core::anyhow::anyhow!(
            "view engine not initialised; call doido_view::init(\"app/views\") at boot"
        )
    })?;
    engine.render(template, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::TemplateEngine;

    struct StubEngine;
    impl TemplateEngine for StubEngine {
        fn render(&self, template: &str, _ctx: &serde_json::Value) -> Result<String> {
            Ok(format!("stub:{template}"))
        }
        fn reload(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn set_then_render_uses_installed_engine() {
        set_engine(Arc::new(StubEngine));
        assert_eq!(
            render("posts/index", &serde_json::json!({})).unwrap(),
            "stub:posts/index"
        );
    }
}
