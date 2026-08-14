use crate::engine::TemplateEngine;
use doido_core::{anyhow::Context as _, Result};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub struct TeraEngine {
    tera: RwLock<tera::Tera>,
    templates_dir: String,
}

impl TeraEngine {
    pub fn new(templates_dir: &str) -> Result<Self> {
        let tera = load(templates_dir)
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
        let ctx = tera::Context::from_serialize(context)
            .map_err(|e| doido_core::anyhow::anyhow!("invalid template context: {e}"))?;
        self.tera
            .read()
            .unwrap()
            .render(&template_name, &ctx)
            .map_err(|e| doido_core::anyhow::anyhow!("template '{}' render failed: {e}", template))
    }

    fn render_named(&self, name: &str, context: &serde_json::Value) -> Result<String> {
        let ctx = tera::Context::from_serialize(context)
            .map_err(|e| doido_core::anyhow::anyhow!("invalid template context: {e}"))?;
        self.tera
            .read()
            .unwrap()
            .render(name, &ctx)
            .map_err(|e| doido_core::anyhow::anyhow!("template '{}' render failed: {e}", name))
    }

    fn reload(&self) -> Result<()> {
        let tera = load(&self.templates_dir)
            .with_context(|| format!("reload failed for {}", self.templates_dir))?;
        *self.tera.write().unwrap() = tera;
        Ok(())
    }
}

/// Load every `*.tera` file under `dir` into a Tera instance, keyed by the file's
/// path relative to `dir` (so `dir/posts/index.html.tera` registers as
/// `posts/index.html.tera`). Tera 2 dropped the glob constructor, so we walk the
/// tree ourselves and add every template in one call (which resolves inheritance
/// across the whole set regardless of insertion order).
///
/// Framework-provided templates (registered via
/// [`crate::global::register_framework_template`] — e.g. `doido-auth`'s built-in
/// auth views) are loaded first, then app templates override any of the same name.
/// Both sets are added in a single `add_raw_templates` call so template
/// inheritance resolves across them (a framework view may `extends` an app layout).
fn load(dir: &str) -> Result<tera::Tera> {
    let base = Path::new(dir);
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    if base.exists() {
        collect(base, base, &mut files)?;
    }

    let mut app: Vec<(String, String)> = Vec::with_capacity(files.len());
    for (path, name) in &files {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading template {}", path.display()))?;
        app.push((name.clone(), content));
    }

    let framework = crate::global::framework_template_snapshot();
    match build(&framework, &app) {
        Ok(tera) => Ok(tera),
        // A framework template that can't resolve (e.g. it `extends` a layout this
        // app doesn't define) must not break the whole engine: fall back to the
        // app's own templates only. App rendering keeps working; the built-in
        // framework view is simply unavailable until the app provides what it needs.
        Err(e) if !framework.is_empty() => {
            doido_core::tracing::warn!(
                "framework templates failed to load ({e}); using app templates only"
            );
            build(&[], &app)
        }
        Err(e) => Err(e),
    }
}

/// Build a Tera instance from `framework` templates (loaded first, overridable)
/// plus `app` templates (override framework ones with the same name). Both sets
/// are added in a single call so inheritance resolves across them.
fn build(framework: &[(String, String)], app: &[(String, String)]) -> Result<tera::Tera> {
    let mut raw: Vec<(&str, &str)> = Vec::with_capacity(framework.len() + app.len());
    for (name, content) in framework {
        if app.iter().any(|(n, _)| n == name) {
            continue; // app template overrides this framework one
        }
        raw.push((name.as_str(), content.as_str()));
    }
    for (name, content) in app {
        raw.push((name.as_str(), content.as_str()));
    }

    let mut tera = tera::Tera::new();
    tera.add_raw_templates(raw)
        .map_err(|e| doido_core::anyhow::anyhow!("{e}"))?;
    Ok(tera)
}

/// Recursively collect `*.tera` files under `dir`, pairing each with its path
/// relative to `base` (forward-slash separated) to use as the Tera template name.
fn collect(base: &Path, dir: &Path, out: &mut Vec<(PathBuf, String)>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect(base, &path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("tera") {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            let name = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.push((path.clone(), name));
        }
    }
    Ok(())
}
