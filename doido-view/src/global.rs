//! Process-global template engine, installed once at boot and reached from
//! request handlers via `Context::render`.
//!
//! Mirrors the framework's other boot-time singletons (the DB pool, inflections):
//! the application installs an engine with [`init`]/[`set_engine`] and controllers
//! render through [`render`] without threading the engine through every call.

use crate::engine::TemplateEngine;
use crate::tera_engine::TeraEngine;
use doido_core::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

static ENGINE: OnceLock<Arc<dyn TemplateEngine>> = OnceLock::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Resets the default initializer state for testing.
#[doc(hidden)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::SeqCst);
}

/// Framework default initializer registered for `view` (priority 18).
///
/// Discovers and initializes the default [`TeraEngine`] over application view directories.
pub fn default_initializer() -> Result<()> {
    if try_engine().is_some() {
        return Ok(());
    }

    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let ctx = doido_core::initializers::detect_execution_context();
    if ctx.is_skip() {
        return Ok(());
    }

    let view_dir = if std::path::Path::new("views").exists() {
        "views"
    } else if std::path::Path::new("app/views").exists() {
        "app/views"
    } else if std::path::Path::new("backend/app/views").exists() {
        "backend/app/views"
    } else {
        "app/views"
    };

    if let Err(e) = init(view_dir) {
        doido_core::tracing::warn!("view default_initializer: {e}");
    }

    Ok(())
}

inventory::submit! {
    doido_core::initializers::FrameworkInitializer {
        name: "view",
        priority: 18,
        init: default_initializer,
    }
}

/// Framework-provided, overridable templates (e.g. `doido-auth`'s built-in auth
/// views). Populated by framework crates *before* [`init`]; the engine loads
/// these first and lets app templates of the same name override them.
static FRAMEWORK_TEMPLATES: OnceLock<Mutex<Vec<(String, String)>>> = OnceLock::new();

fn framework_templates() -> &'static Mutex<Vec<(String, String)>> {
    FRAMEWORK_TEMPLATES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Registers an overridable, framework-provided template so `Context::render`
/// resolves it even when the app has not written its own copy. `name` is the
/// Tera template name including the extension (e.g. `"auth/sign_in.html.tera"`);
/// `content` is the raw template source (typically `include_str!`).
///
/// Call this *before* [`init`]/[`set_engine`]. An app template loaded from the
/// view directory with the same `name` overrides the framework one. Idempotent:
/// re-registering the same `name` is a no-op.
pub fn register_framework_template(name: &str, content: &str) {
    let mut templates = framework_templates().lock().unwrap();
    if templates.iter().any(|(n, _)| n == name) {
        return;
    }
    templates.push((name.to_string(), content.to_string()));
    drop(templates);
    if let Some(engine) = try_engine() {
        let _ = engine.reload();
    }
}

/// Snapshot of the registered framework templates as `(name, content)` pairs.
/// Used by the Tera engine at load time; app templates override by name.
pub fn framework_template_snapshot() -> Vec<(String, String)> {
    framework_templates().lock().unwrap().clone()
}

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

/// Render a specific format `variant` of `template` (e.g. `("mailers/x/welcome",
/// "text")` renders `mailers/x/welcome.text.tera`). Used for mailer html/text
/// parts, where `render` — which always appends `.html.tera` — is not enough.
pub fn render_variant(
    template: &str,
    variant: &str,
    context: &serde_json::Value,
) -> Result<String> {
    let engine = ENGINE.get().ok_or_else(|| {
        doido_core::anyhow::anyhow!(
            "view engine not initialised; call doido_view::init(\"app/views\") at boot"
        )
    })?;
    engine.render_named(&format!("{template}.{variant}.tera"), context)
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
