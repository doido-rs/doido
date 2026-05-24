//! New application skeleton rendered from embedded files under `templates/app/`.
//! Placeholders: `{doido_name}`, `{doido_db_url}`, `{doido_sqlx_feature}`,
//! `{doido_version}`, `{doido_controller_version}` (semver pins captured at compile time).

use crate::generator::{GeneratedFile, Generator};
use doido_core::{anyhow, Result};
use include_dir::{include_dir, Dir, DirEntry};

/// Embedded filesystem tree merged at compile time from `templates/app`.
static APP_TEMPLATE_DIR: Dir<'static> =
    include_dir!("$CARGO_MANIFEST_DIR/templates/app");

struct TemplateContext<'a> {
    name: &'a str,
    db_url: String,
    sqlx_feature: &'a str,
}

fn substitute_template(template: &str, ctx: &TemplateContext<'_>) -> String {
    template
        .replace("{doido_name}", ctx.name)
        .replace("{doido_db_url}", &ctx.db_url)
        .replace("{doido_sqlx_feature}", ctx.sqlx_feature)
        .replace("{doido_version}", crate::TEMPLATE_PINNED_DOIDO_VERSION)
        .replace(
            "{doido_controller_version}",
            crate::TEMPLATE_PINNED_DOIDO_CONTROLLER_VERSION,
        )
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
                // `include_dir` stores paths relative to the embedded root (`templates/app/`)
                // for every file, including nested paths like `src/main.rs`.
                let relative = f.path();
                let raw = f.contents_utf8().ok_or_else(|| {
                    anyhow::anyhow!(
                        "template file '{}' is not valid UTF-8",
                        relative.display()
                    )
                })?;
                let rendered = substitute_template(raw, ctx);
                let disk_path = format!(
                    "{}/{}",
                    app_name,
                    relative.to_string_lossy().replace('\\', "/")
                );
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

        let db_url = match database {
            "postgres" => format!("postgres://localhost/{name}_development"),
            "mysql" => format!("mysql://localhost/{name}_development"),
            _ => "sqlite://db/development.db".to_string(),
        };

        let sqlx_feature = match database {
            "postgres" => "postgres",
            "mysql" => "mysql",
            _ => "sqlite",
        };

        let ctx = TemplateContext {
            name,
            db_url,
            sqlx_feature,
        };

        let mut files = Vec::new();
        collect_from_dir(&APP_TEMPLATE_DIR, &ctx, name, &mut files)?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }
}
