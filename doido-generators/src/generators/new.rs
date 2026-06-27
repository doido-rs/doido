//! New application skeleton rendered from embedded files under `templates/new/`.
//! Placeholders: `{doido_name}`, `{doido_db_url}`, `{doido_sqlx_feature}`,
//! `{doido_path}` (absolute workspace root captured at compile time, used for
//! local `doido-*` path dependencies).
//!
//! Template files carrying a trailing `.template` suffix (e.g. `Cargo.toml.template`)
//! have the suffix stripped on output; the suffix keeps `cargo package` from treating
//! `templates/new/` as a nested crate and excluding it from the published tarball.

use crate::generator::{GeneratedFile, Generator};
use doido_core::{anyhow, Result};
use include_dir::{include_dir, Dir, DirEntry};

/// Embedded filesystem tree merged at compile time from `templates/new`.
static APP_TEMPLATE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/new");

struct TemplateContext<'a> {
    name: &'a str,
    db_url: String,
    db_url_test: String,
    db_url_production: String,
    sqlx_feature: &'a str,
}

fn substitute_template(template: &str, ctx: &TemplateContext<'_>) -> String {
    template
        .replace("{doido_name}", ctx.name)
        .replace("{doido_db_url_test}", &ctx.db_url_test)
        .replace("{doido_db_url_production}", &ctx.db_url_production)
        .replace("{doido_db_url}", &ctx.db_url)
        .replace("{doido_sqlx_feature}", ctx.sqlx_feature)
        .replace("{doido_path}", crate::TEMPLATE_WORKSPACE_PATH)
}

fn collect_from_dir(
    dir: &Dir<'_>,
    ctx: &TemplateContext<'_>,
    app_name: &str,
    out: &mut Vec<GeneratedFile>,
) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(sub) => collect_from_dir(sub, ctx, app_name, out)?,
            DirEntry::File(f) => {
                // `include_dir` stores paths relative to the embedded root (`templates/new/`)
                // for every file, including nested paths like `src/main.rs`.
                let relative = f.path();
                let raw = f.contents_utf8().ok_or_else(|| {
                    anyhow::anyhow!("template file '{}' is not valid UTF-8", relative.display())
                })?;
                let rendered = substitute_template(raw, ctx);
                // Template manifests are stored with a trailing `.template` suffix
                // (e.g. `Cargo.toml.template`) so `cargo package` doesn't mistake
                // `templates/app/` for a nested crate and drop it from the tarball.
                // Strip the suffix when writing the generated app to disk.
                let relative = relative.to_string_lossy().replace('\\', "/");
                let relative = relative.strip_suffix(".template").unwrap_or(&relative);
                let disk_path = format!("{app_name}/{relative}");
                out.push(GeneratedFile {
                    path: disk_path,
                    content: rendered,
                });
            }
        }
    }
    Ok(())
}

pub struct ProjectGenerator;

impl Generator for ProjectGenerator {
    fn name(&self) -> &str {
        "new"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("new generator requires a name argument"))?;

        let database = args
            .iter()
            .find(|a| a.starts_with("--database="))
            .and_then(|a| a.split_once('=').map(|(_, v)| v))
            .unwrap_or("sqlite");

        match database {
            "sqlite" | "postgres" | "mysql" => {}
            other => {
                return Err(anyhow::anyhow!(
                    "Unknown database: {}. Use sqlite, postgres, or mysql.",
                    other
                ));
            }
        }

        let (db_url, db_url_test, db_url_production) = match database {
            "postgres" => (
                format!("postgres://localhost/{name}_development"),
                format!("postgres://localhost/{name}_test"),
                format!("postgres://localhost/{name}_production"),
            ),
            "mysql" => (
                format!("mysql://localhost/{name}_development"),
                format!("mysql://localhost/{name}_test"),
                format!("mysql://localhost/{name}_production"),
            ),
            _ => (
                "sqlite://db/development.db".to_string(),
                "sqlite://db/test.db".to_string(),
                "sqlite://db/production.db".to_string(),
            ),
        };

        let sqlx_feature = match database {
            "postgres" => "postgres",
            "mysql" => "mysql",
            _ => "sqlite",
        };

        let ctx = TemplateContext {
            name,
            db_url,
            db_url_test,
            db_url_production,
            sqlx_feature,
        };

        let mut files = Vec::new();
        collect_from_dir(&APP_TEMPLATE_DIR, &ctx, name, &mut files)?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }
}
