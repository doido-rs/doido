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

/// Default local connection parameters for a server backend.
struct DbDefaults {
    /// URL scheme (`postgres` / `mysql`).
    scheme: &'static str,
    /// Default superuser for a local install.
    user: &'static str,
    /// Default development/test password (placeholder in production).
    password: &'static str,
    /// Default listening port.
    port: u16,
}

/// Returns the default connection parameters for `postgres`/`mysql`, or `None`
/// for file-based backends (sqlite) that carry no user/host/port.
fn db_defaults(backend: &str) -> Option<DbDefaults> {
    match backend {
        "postgres" => Some(DbDefaults {
            scheme: "postgres",
            user: "postgres",
            password: "postgres",
            port: 5432,
        }),
        "mysql" => Some(DbDefaults {
            scheme: "mysql",
            user: "root",
            password: "password",
            port: 3306,
        }),
        _ => None,
    }
}

/// Builds the `database.url` for one environment of a generated app.
///
/// Server backends (postgres/mysql) include the default user, password, host
/// and port so the generated config is close to a working local setup, e.g.
/// `postgres://postgres:postgres@localhost:5432/blog_development`. sqlite uses a
/// bare file path. In **production** the password is a `CHANGE_ME` placeholder
/// that must be overridden (e.g. via the `DATABASE_URL` env var) — real
/// credentials are never baked into the generated repo.
///
/// Note: the default credentials contain no URL-reserved characters, so no
/// percent-encoding is needed. That would change if custom passwords were ever
/// accepted here.
fn default_database_url(backend: &str, name: &str, env: &str) -> String {
    match db_defaults(backend) {
        Some(d) => {
            let password = if env == "production" {
                "CHANGE_ME"
            } else {
                d.password
            };
            format!(
                "{}://{}:{}@localhost:{}/{}_{}",
                d.scheme, d.user, password, d.port, name, env
            )
        }
        None => format!("sqlite://db/{env}.db"),
    }
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

        let db_url = default_database_url(database, name, "development");
        let db_url_test = default_database_url(database, name, "test");
        let db_url_production = default_database_url(database, name, "production");

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

#[cfg(test)]
mod tests {
    use super::default_database_url;

    #[test]
    fn postgres_url_has_default_user_password_and_port() {
        assert_eq!(
            default_database_url("postgres", "blog", "development"),
            "postgres://postgres:postgres@localhost:5432/blog_development"
        );
    }

    #[test]
    fn mysql_url_has_default_user_password_and_port() {
        assert_eq!(
            default_database_url("mysql", "store", "test"),
            "mysql://root:password@localhost:3306/store_test"
        );
    }

    #[test]
    fn production_password_is_a_placeholder() {
        assert_eq!(
            default_database_url("postgres", "blog", "production"),
            "postgres://postgres:CHANGE_ME@localhost:5432/blog_production"
        );
        assert_eq!(
            default_database_url("mysql", "store", "production"),
            "mysql://root:CHANGE_ME@localhost:3306/store_production"
        );
    }

    #[test]
    fn sqlite_stays_a_bare_file_path() {
        assert_eq!(
            default_database_url("sqlite", "blog", "development"),
            "sqlite://db/development.db"
        );
    }
}
