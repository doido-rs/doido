//! New application skeleton rendered from embedded files under `templates/new/`.
//! Placeholders: `{doido_name}`, `{doido_db_url}`, `{doido_sqlx_feature}`,
//! `{doido_path}` (absolute workspace root when the running binary lives inside a
//! local checkout), and the per-crate dependency specs `{doido_dep}` /
//! `{doido_controller_dep}` / `{doido_model_dep}` which render as a local `path`
//! dep when the binary runs from a development checkout or a crates.io `version`
//! dep matching this binary's release otherwise (see [`DependencyMode`]).
//!
//! The optional doido-cable example lives inside this same template. Its channel
//! files sit under `templates/new/app/channels/` (skipped unless `--cable` is
//! passed), and the `{doido_cable_deps}` / `{doido_channels_module}` /
//! `{doido_cable_readme}` placeholders in `Cargo.toml`, `src/main.rs`, and
//! `README.md` render to their wiring when `--cable` is set, or to nothing
//! otherwise (see [`substitute_template`]).
//!
//! Template files carrying a trailing `.template` suffix (e.g. `Cargo.toml.template`)
//! have the suffix stripped on output; the suffix keeps `cargo package` from treating
//! `templates/new/` as a nested crate and excluding it from the published tarball.

use crate::dev_workspace::DependencyMode;
use crate::generator::{GeneratedFile, Generator};
use crate::new_options::{parse_cache, parse_database, parse_jobs, CacheBackend, JobsBackend};
use doido_core::{anyhow, Result};
use include_dir::{include_dir, Dir, DirEntry};

/// Embedded filesystem tree merged at compile time from `templates/new`.
static APP_TEMPLATE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/new");

/// Template subtree holding the doido-cable example. Files here are skipped
/// unless `--cable` is passed.
const CABLE_TEMPLATE_PREFIX: &str = "app/channels/";

/// `mod channels;` include spliced into `src/main.rs` when `--cable` is passed.
const CABLE_MODULE_INCLUDE: &str = "\n#[path = \"../app/channels/mod.rs\"]\nmod channels;\n";

/// README section explaining how the generated doido-cable example is wired.
/// `{doido_name}` is substituted like any other template token.
const CABLE_README_SECTION: &str = r#"
## Real-time with doido-cable

This app was generated with `--cable`, so it includes:

- the `doido-cable` (and `async-trait`) dependencies in `Cargo.toml`;
- an example channel at `app/channels/chat_channel.rs`, registered in
  `app/channels/mod.rs` and wired into the crate via `mod channels;` in
  `src/main.rs`.

A channel implements the `Channel` trait — `subscribed`, `unsubscribed`, and
`received` — and broadcasts to other clients through a shared `Cable` handle over
a pub/sub backend (`MemoryPubSub` by default; Redis/DB are swappable). See the
`#[tokio::test]` in `app/channels/chat_channel.rs` for a runnable
subscribe → broadcast → receive round-trip:

```sh
cargo test --bin {doido_name} chat
```
"#;

struct TemplateContext<'a> {
    name: &'a str,
    db_url: String,
    db_url_test: String,
    db_url_production: String,
    sqlx_feature: &'a str,
    cable: bool,
    dep_mode: DependencyMode,
    doido_dep: String,
    doido_jobs_dep: String,
    cache_section: String,
    jobs_section: String,
    compose_services: String,
    compose_depends_on: String,
    compose_database_url: String,
    compose_env_extras: String,
    compose_web_volumes: String,
}

/// Renders a complete Cargo inline-table dependency for a first-party `doido-*` crate.
fn doido_dependency(mode: &DependencyMode, subdir: &str, features: &str) -> String {
    dependency_spec(
        mode.use_path,
        &mode.workspace_path,
        mode.version,
        subdir,
        features,
    )
}

/// `features` is an optional suffix such as `, features = ["cache-redis"]` (empty when none).
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

fn flag_value<'a>(args: &'a [&str], prefix: &str, default: &'a str) -> &'a str {
    args.iter()
        .find(|a| a.starts_with(prefix))
        .and_then(|a| a.split_once('=').map(|(_, v)| v))
        .unwrap_or(default)
}

fn doido_features(cache: CacheBackend) -> String {
    match cache {
        CacheBackend::Redis => ", features = [\"cache-redis\"]".to_string(),
        CacheBackend::Memcache => ", features = [\"cache-memcache\"]".to_string(),
        CacheBackend::Memory => String::new(),
    }
}

fn doido_jobs_features(jobs: JobsBackend) -> String {
    match jobs {
        JobsBackend::Db => ", features = [\"jobs-db\"]".to_string(),
        JobsBackend::Redis => ", features = [\"jobs-redis\"]".to_string(),
        JobsBackend::Memory => String::new(),
    }
}

fn render_cache_section(cache: CacheBackend, name: &str) -> String {
    match cache {
        CacheBackend::Memory => "cache:\n  type: memory\n".to_string(),
        CacheBackend::Redis => format!(
            "cache:\n  type: redis\n  endpoint: redis://127.0.0.1:6379\n  namespace: {name}\n"
        ),
        CacheBackend::Memcache => format!(
            "cache:\n  type: memcache\n  endpoint: memcache://127.0.0.1:11211\n  namespace: {name}\n"
        ),
    }
}

fn render_jobs_section(jobs: JobsBackend, name: &str) -> String {
    match jobs {
        JobsBackend::Memory => "jobs:\n  type: memory\n".to_string(),
        JobsBackend::Db => {
            "jobs:\n  type: db\n  queues: [default]\n  concurrency: 5\n".to_string()
        }
        JobsBackend::Redis => format!(
            "jobs:\n  type: redis\n  queues: [default]\n  concurrency: 5\n  redis:\n    url: redis://127.0.0.1:6379\n    namespace: {name}:jobs\n"
        ),
    }
}

fn needs_redis(cable: bool, cache: CacheBackend, jobs: JobsBackend) -> bool {
    cable || cache == CacheBackend::Redis || jobs == JobsBackend::Redis
}

fn compose_database_url_for_docker(database: &str, name: &str) -> String {
    match database {
        "postgres" => format!("postgres://postgres:postgres@postgres:5432/{name}_development"),
        "mysql" => format!("mysql://root:password@mysql:3306/{name}_development"),
        _ => "sqlite://db/development.db".to_string(),
    }
}

fn compose_postgres_service(name: &str) -> String {
    format!(
        r#"  postgres:
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: {name}_development
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 2s
      timeout: 3s
      retries: 15"#
    )
}

fn compose_mysql_service(name: &str) -> String {
    format!(
        r#"  mysql:
    image: mysql:lts
    environment:
      MYSQL_ROOT_PASSWORD: password
      MYSQL_DATABASE: {name}_development
    ports:
      - "3306:3306"
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost"]
      interval: 2s
      timeout: 3s
      retries: 15"#
    )
}

fn compose_redis_service() -> &'static str {
    r#"  redis:
    image: redis:8-alpine
    ports:
      - "6379:6379"
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 2s
      timeout: 3s
      retries: 15"#
}

fn compose_memcache_service() -> &'static str {
    r#"  memcache:
    image: memcached:1.6-alpine
    ports:
      - "11211:11211""#
}

fn compose_services(
    database: &str,
    name: &str,
    cable: bool,
    cache: CacheBackend,
    jobs: JobsBackend,
) -> String {
    let mut parts = Vec::new();
    match database {
        "postgres" => parts.push(compose_postgres_service(name)),
        "mysql" => parts.push(compose_mysql_service(name)),
        _ => {}
    }
    if needs_redis(cable, cache, jobs) {
        parts.push(compose_redis_service().to_string());
    }
    if cache == CacheBackend::Memcache {
        parts.push(compose_memcache_service().to_string());
    }
    parts.join("\n\n")
}

fn compose_depends_on(
    database: &str,
    cable: bool,
    cache: CacheBackend,
    jobs: JobsBackend,
) -> String {
    let mut deps = Vec::new();
    match database {
        "postgres" => deps.push("      postgres:\n        condition: service_healthy"),
        "mysql" => deps.push("      mysql:\n        condition: service_healthy"),
        _ => {}
    }
    if needs_redis(cable, cache, jobs) {
        deps.push("      redis:\n        condition: service_healthy");
    }
    if deps.is_empty() {
        String::new()
    } else {
        format!("    depends_on:\n{}", deps.join("\n"))
    }
}

fn compose_env_extras(cache: CacheBackend, jobs: JobsBackend) -> String {
    let mut lines = Vec::new();
    match cache {
        CacheBackend::Redis => lines.push("      CACHE__ENDPOINT: redis://redis:6379"),
        CacheBackend::Memcache => lines.push("      CACHE__ENDPOINT: memcache://memcache:11211"),
        CacheBackend::Memory => {}
    }
    if jobs == JobsBackend::Redis {
        lines.push("      JOBS__REDIS__URL: redis://redis:6379");
    }
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n")
    }
}

fn compose_web_volumes(database: &str) -> String {
    if database == "sqlite" {
        "      - ./db:/app/db\n".to_string()
    } else {
        String::new()
    }
}

fn substitute_template(template: &str, ctx: &TemplateContext<'_>) -> String {
    let (cable_deps, cable_module, cable_readme) = if ctx.cable {
        (
            format!(
                "doido-cable = {}\nasync-trait = \"0.1\"\n",
                doido_dependency(&ctx.dep_mode, "doido-cable", "")
            ),
            CABLE_MODULE_INCLUDE.to_string(),
            CABLE_README_SECTION.replace("{doido_name}", ctx.name),
        )
    } else {
        (String::new(), String::new(), String::new())
    };

    template
        .replace("{doido_name}", ctx.name)
        .replace("{doido_db_url_test}", &ctx.db_url_test)
        .replace("{doido_db_url_production}", &ctx.db_url_production)
        .replace("{doido_db_url}", &ctx.db_url)
        .replace("{doido_sqlx_feature}", ctx.sqlx_feature)
        .replace("{doido_dep}", &ctx.doido_dep)
        .replace(
            "{doido_core_dep}",
            &doido_dependency(&ctx.dep_mode, "doido-core", ""),
        )
        .replace(
            "{doido_controller_dep}",
            &doido_dependency(&ctx.dep_mode, "doido-controller", ""),
        )
        .replace("{doido_jobs_dep}", &ctx.doido_jobs_dep)
        .replace(
            "{doido_mailer_dep}",
            &doido_dependency(&ctx.dep_mode, "doido-mailer", ""),
        )
        .replace(
            "{doido_model_dep}",
            &doido_dependency(&ctx.dep_mode, "doido-model", ""),
        )
        .replace("{doido_cable_deps}", &cable_deps)
        .replace("{doido_channels_module}", &cable_module)
        .replace("{doido_cable_readme}", &cable_readme)
        .replace("{doido_cache_section}", &ctx.cache_section)
        .replace("{doido_jobs_section}", &ctx.jobs_section)
        .replace("{doido_compose_services}", &ctx.compose_services)
        .replace("{doido_compose_depends_on}", &ctx.compose_depends_on)
        .replace("{doido_compose_database_url}", &ctx.compose_database_url)
        .replace("{doido_compose_env_extras}", &ctx.compose_env_extras)
        .replace("{doido_compose_web_volumes}", &ctx.compose_web_volumes)
        .replace("{doido_path}", &ctx.dep_mode.workspace_path)
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
                let relative = f.path();
                if !ctx.cable && relative.starts_with(CABLE_TEMPLATE_PREFIX) {
                    continue;
                }
                let raw = f.contents_utf8().ok_or_else(|| {
                    anyhow::anyhow!("template file '{}' is not valid UTF-8", relative.display())
                })?;
                let rendered = substitute_template(raw, ctx);
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

struct DbDefaults {
    scheme: &'static str,
    user: &'static str,
    password: &'static str,
    port: u16,
}

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

        let database = parse_database(flag_value(args, "--database=", "sqlite"))?;
        let cache = parse_cache(flag_value(args, "--cache=", "memory"))?;
        let jobs = parse_jobs(flag_value(args, "--jobs=", "memory"))?;
        let cable = args.contains(&"--cable");

        let database = database.as_str();
        let db_url = default_database_url(database, name, "development");
        let db_url_test = default_database_url(database, name, "test");
        let db_url_production = default_database_url(database, name, "production");

        let sqlx_feature = match database {
            "postgres" => "postgres",
            "mysql" => "mysql",
            _ => "sqlite",
        };

        let dep_mode = DependencyMode::resolve();

        let ctx = TemplateContext {
            name,
            db_url,
            db_url_test,
            db_url_production,
            sqlx_feature,
            cable,
            doido_dep: doido_dependency(&dep_mode, "doido", &doido_features(cache)),
            doido_jobs_dep: doido_dependency(&dep_mode, "doido-jobs", &doido_jobs_features(jobs)),
            dep_mode,
            cache_section: render_cache_section(cache, name),
            jobs_section: render_jobs_section(jobs, name),
            compose_services: compose_services(database, name, cable, cache, jobs),
            compose_depends_on: compose_depends_on(database, cable, cache, jobs),
            compose_database_url: compose_database_url_for_docker(database, name),
            compose_env_extras: compose_env_extras(cache, jobs),
            compose_web_volumes: compose_web_volumes(database),
        };

        let mut files = Vec::new();
        collect_from_dir(&APP_TEMPLATE_DIR, &ctx, name, &mut files)?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_options::{parse_cache, parse_jobs};

    #[test]
    fn local_builds_emit_path_dependencies() {
        assert_eq!(
            dependency_spec(true, "/home/dev/doido", "0.0.6", "doido", ""),
            "{ path = \"/home/dev/doido/doido\" }"
        );
    }

    #[test]
    fn published_builds_emit_version_dependencies() {
        assert_eq!(
            dependency_spec(false, "/irrelevant", "0.0.6", "doido", ""),
            "{ version = \"0.0.6\" }"
        );
    }

    #[test]
    fn path_dependency_with_features_stays_inside_inline_table() {
        assert_eq!(
            dependency_spec(
                true,
                "/home/dev/doido",
                "0.0.6",
                "doido",
                ", features = [\"cache-redis\"]",
            ),
            "{ path = \"/home/dev/doido/doido\", features = [\"cache-redis\"] }"
        );
    }

    #[test]
    fn published_dependency_with_features_stays_inside_inline_table() {
        assert_eq!(
            dependency_spec(
                false,
                "/irrelevant",
                "0.0.6",
                "doido-jobs",
                ", features = [\"jobs-redis\"]",
            ),
            "{ version = \"0.0.6\", features = [\"jobs-redis\"] }"
        );
    }

    fn assert_cargo_toml_parses(cargo_toml: &str) {
        cargo_toml
            .parse::<toml::Table>()
            .expect("valid Cargo.toml TOML");
    }

    fn minimal_cargo_with_doido_line(doido_line: &str) -> String {
        format!(
            r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
{doido_line}
"#
        )
    }

    #[test]
    fn published_cache_redis_line_is_valid_toml() {
        let line = format!(
            "doido = {}",
            dependency_spec(
                false,
                "/irrelevant",
                "0.0.9",
                "doido",
                ", features = [\"cache-redis\"]",
            )
        );
        assert!(!line.contains("path ="));
        assert!(line.contains("version = \"0.0.9\""));
        assert!(line.contains("cache-redis"));
        assert_cargo_toml_parses(&minimal_cargo_with_doido_line(&line));
    }

    #[test]
    fn path_jobs_redis_line_is_valid_toml() {
        let line = format!(
            "doido-jobs = {}",
            dependency_spec(
                true,
                "/home/dev/doido",
                "0.0.9",
                "doido-jobs",
                ", features = [\"jobs-redis\"]",
            )
        );
        assert!(line.contains("path = \"/home/dev/doido/doido-jobs\""));
        assert!(line.contains("jobs-redis"));
        assert_cargo_toml_parses(&minimal_cargo_with_doido_line(&line));
    }

    #[test]
    fn postgres_url_has_default_user_password_and_port() {
        assert_eq!(
            default_database_url("postgres", "blog", "development"),
            "postgres://postgres:postgres@localhost:5432/blog_development"
        );
    }

    #[test]
    fn production_password_is_a_placeholder() {
        assert_eq!(
            default_database_url("postgres", "blog", "production"),
            "postgres://postgres:CHANGE_ME@localhost:5432/blog_production"
        );
    }

    #[test]
    fn sqlite_stays_a_bare_file_path() {
        assert_eq!(
            default_database_url("sqlite", "blog", "development"),
            "sqlite://db/development.db"
        );
    }

    #[test]
    fn parse_cache_accepts_memcached_alias() {
        assert_eq!(parse_cache("memcached").unwrap(), CacheBackend::Memcache);
    }

    #[test]
    fn parse_jobs_accepts_database_alias() {
        assert_eq!(parse_jobs("database").unwrap(), JobsBackend::Db);
    }

    #[test]
    fn compose_includes_redis_for_cache_redis() {
        let svc = compose_services(
            "sqlite",
            "app",
            false,
            CacheBackend::Redis,
            JobsBackend::Memory,
        );
        assert!(svc.contains("redis:8-alpine"));
        assert!(!svc.contains("postgres:"));
    }

    #[test]
    fn compose_includes_memcache_for_cache_memcache() {
        let svc = compose_services(
            "sqlite",
            "app",
            false,
            CacheBackend::Memcache,
            JobsBackend::Memory,
        );
        assert!(svc.contains("memcached:1.6-alpine"));
        assert!(!svc.contains("redis:"));
    }

    #[test]
    fn compose_deduplicates_redis_when_cable_and_jobs_redis() {
        let svc = compose_services(
            "sqlite",
            "app",
            true,
            CacheBackend::Memory,
            JobsBackend::Redis,
        );
        assert_eq!(svc.matches("image: redis:").count(), 1);
    }

    #[test]
    fn compose_database_url_uses_docker_hostnames() {
        assert_eq!(
            compose_database_url_for_docker("postgres", "blog"),
            "postgres://postgres:postgres@postgres:5432/blog_development"
        );
    }
}
