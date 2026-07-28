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
fn load(dir: &str) -> Result<tera::Tera> {
    let base = Path::new(dir);
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    if base.exists() {
        collect(base, base, &mut files)?;
    }
    let mut tera = tera::Tera::new();
    tera.add_template_files(files.iter().map(|(path, name)| (path, Some(name.as_str()))))
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
