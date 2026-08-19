//! `doido extension <name>` — scaffolds a publishable extension crate
//! (`doido-<snake>/`) from embedded templates under `templates/extension/`.

use crate::dev_workspace::DependencyMode;
use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_pascal, to_snake};
use doido_core::{anyhow::anyhow, Result};
use include_dir::{include_dir, Dir, DirEntry};
use std::path::Path;

/// Embedded filesystem tree merged at compile time from `templates/extension`.
static EXTENSION_TEMPLATE_DIR: Dir<'static> =
    include_dir!("$CARGO_MANIFEST_DIR/templates/extension");

struct TemplateContext<'a> {
    crate_name: &'a str,
    snake: &'a str,
    pascal: &'a str,
    crate_ident: String,
    dep_mode: DependencyMode,
    doido_dep: String,
    doido_core_dep: String,
    doido_controller_dep: String,
}

struct ExtensionNames {
    crate_name: String,
    snake: String,
    pascal: String,
    crate_ident: String,
}

/// Normalise user input into `doido-<snake>` and related tokens.
fn normalize_extension_name(name: &str) -> ExtensionNames {
    let raw = name.trim();
    let snake = to_snake(raw.strip_prefix("doido-").unwrap_or(raw));
    let pascal = to_pascal(&snake);
    let crate_name = format!("doido-{snake}");
    let crate_ident = crate_name.replace('-', "_");
    ExtensionNames {
        crate_name,
        snake,
        pascal,
        crate_ident,
    }
}

fn doido_dependency(mode: &DependencyMode, subdir: &str, features: &str) -> String {
    dependency_spec(
        mode.use_path,
        &mode.workspace_path,
        mode.version,
        subdir,
        features,
    )
}

fn dependency_spec(
    use_path: bool,
    workspace_path: &str,
    version: &str,
    subdir: &str,
    features: &str,
) -> String {
    let inner = if use_path {
        format!("path = \"{workspace_path}/{subdir}\"")
    } else {
        format!("version = \"{version}\"")
    };
    format!("{{ {inner}{features} }}")
}

fn substitute_template(template: &str, ctx: &TemplateContext<'_>) -> String {
    template
        .replace("{doido_ext_name}", ctx.crate_name)
        .replace("{doido_ext_snake}", ctx.snake)
        .replace("{doido_ext_pascal}", ctx.pascal)
        .replace("{doido_ext_crate_ident}", &ctx.crate_ident)
        .replace("{doido_dep}", &ctx.doido_dep)
        .replace("{doido_core_dep}", &ctx.doido_core_dep)
        .replace("{doido_controller_dep}", &ctx.doido_controller_dep)
        .replace("{doido_path}", &ctx.dep_mode.workspace_path)
}

fn collect_from_dir(
    dir: &Dir<'_>,
    ctx: &TemplateContext<'_>,
    crate_dir: &str,
    out: &mut Vec<GeneratedFile>,
) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(sub) => collect_from_dir(sub, ctx, crate_dir, out)?,
            DirEntry::File(f) => {
                let relative = f.path();
                let raw = f.contents_utf8().ok_or_else(|| {
                    anyhow!("template file '{}' is not valid UTF-8", relative.display())
                })?;
                let rendered = substitute_template(raw, ctx);
                let relative = relative.to_string_lossy().replace('\\', "/");
                let relative = relative.strip_suffix(".template").unwrap_or(&relative);
                let disk_path = format!("{crate_dir}/{relative}");
                out.push(GeneratedFile {
                    path: disk_path,
                    content: rendered,
                });
            }
        }
    }
    Ok(())
}

pub struct ExtensionGenerator;

impl Generator for ExtensionGenerator {
    fn name(&self) -> &str {
        "extension"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args
            .first()
            .copied()
            .ok_or_else(|| anyhow!("extension generator requires a name argument"))?;

        let names = normalize_extension_name(name);
        let crate_dir = names.crate_name.clone();

        if Path::new(&crate_dir).exists() {
            return Err(anyhow!(
                "directory '{crate_dir}' already exists — choose another name or remove it"
            ));
        }

        let dep_mode = DependencyMode::resolve();
        let ctx = TemplateContext {
            crate_name: &names.crate_name,
            snake: &names.snake,
            pascal: &names.pascal,
            crate_ident: names.crate_ident,
            doido_dep: doido_dependency(
                &dep_mode,
                "doido",
                ", default-features = false, features = [\"sqlite\"]",
            ),
            doido_core_dep: doido_dependency(&dep_mode, "doido-core", ""),
            doido_controller_dep: doido_dependency(
                &dep_mode,
                "doido-controller",
                ", features = [\"sqlite\"]",
            ),
            dep_mode,
        };

        let mut files = Vec::new();
        collect_from_dir(&EXTENSION_TEMPLATE_DIR, &ctx, &crate_dir, &mut files)?;
        if files.is_empty() {
            return Err(anyhow!("extension templates produced no files"));
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_doido_prefix_and_underscores() {
        let names = normalize_extension_name("doido-payments");
        assert_eq!(names.crate_name, "doido-payments");
        assert_eq!(names.snake, "payments");
        assert_eq!(names.pascal, "Payments");
        assert_eq!(names.crate_ident, "doido_payments");
    }

    #[test]
    fn normalize_accepts_pascal_case() {
        let names = normalize_extension_name("BlogAnalytics");
        assert_eq!(names.crate_name, "doido-blog_analytics");
        assert_eq!(names.pascal, "BlogAnalytics");
    }

    #[test]
    fn generated_cargo_toml_is_valid() {
        let files = ExtensionGenerator
            .generate(&["Payments"])
            .expect("generate extension");
        let cargo = files
            .iter()
            .find(|f| f.path == "doido-payments/Cargo.toml")
            .expect("Cargo.toml");
        cargo.content.parse::<toml::Table>().expect("valid toml");
        assert!(cargo.content.contains("name = \"doido-payments\""));
    }
}
