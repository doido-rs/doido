//! Single source of truth for built-in generator templates, plus project-level
//! override resolution.
//!
//! Every generator (except `new`) sources its template strings through
//! [`get`], which prefers a project-local `templates/<rel>` file over the
//! built-in default. The same `(rel, default)` table powers the
//! `doido generate templates` eject command (see [`builtin_templates`]).

use std::path::{Path, PathBuf};

// --- generator templates, embedded from `templates/<generator>/` ---
// Tokens (`{snake}`, `{pascal}`) are substituted by each generator after
// resolution, so a project override file uses the exact same tokens.

const CONTROLLER: &str = include_str!("../templates/controller/controller.rs.template");
const CONTROLLER_TEST: &str = include_str!("../templates/controller/controller_test.rs.template");
const JOB: &str = include_str!("../templates/job/job.rs.template");
const JOB_TEST: &str = include_str!("../templates/job/job_test.rs.template");
const MAILER: &str = include_str!("../templates/mailer/mailer.rs.template");
const MAILER_TEST: &str = include_str!("../templates/mailer/mailer_test.rs.template");
const CHANNEL: &str = include_str!("../templates/channel/channel.rs.template");
const CHANNEL_TEST: &str = include_str!("../templates/channel/channel_test.rs.template");
const MIGRATION: &str = include_str!("../templates/migration/migration.rs.template");
const MIGRATION_TEST: &str = include_str!("../templates/migration/migration_test.rs.template");

// --- file-based defaults (scaffold + model), kept embedded as before ---
const MODEL: &str = include_str!("../templates/models/model.rs.template");
const MODEL_MIGRATION: &str = include_str!("../templates/models/migration.rs.template");
const MODEL_TEST: &str = include_str!("../templates/models/model_test.rs.template");
const SCAFFOLD_CONTROLLER_HTML: &str =
    include_str!("../templates/scaffold/controller_html.rs.template");
const SCAFFOLD_CONTROLLER_API: &str =
    include_str!("../templates/scaffold/controller_api.rs.template");
const SCAFFOLD_CONTROLLER_TEST: &str =
    include_str!("../templates/scaffold/controller_test.rs.template");
const VIEW_INDEX: &str = include_str!("../templates/scaffold/views/index.html.tera");
const VIEW_SHOW: &str = include_str!("../templates/scaffold/views/show.html.tera");
const VIEW_NEW: &str = include_str!("../templates/scaffold/views/new.html.tera");
const VIEW_EDIT: &str = include_str!("../templates/scaffold/views/edit.html.tera");
const VIEW_FORM: &str = include_str!("../templates/scaffold/views/_form.html.tera");

/// `(rel_path, default_content)` for every overridable built-in template. The
/// rel path mirrors the project override layout under `templates/`.
const BUILTIN: &[(&str, &str)] = &[
    ("controller/controller.rs.template", CONTROLLER),
    ("controller/controller_test.rs.template", CONTROLLER_TEST),
    ("job/job.rs.template", JOB),
    ("job/job_test.rs.template", JOB_TEST),
    ("mailer/mailer.rs.template", MAILER),
    ("mailer/mailer_test.rs.template", MAILER_TEST),
    ("channel/channel.rs.template", CHANNEL),
    ("channel/channel_test.rs.template", CHANNEL_TEST),
    ("migration/migration.rs.template", MIGRATION),
    ("migration/migration_test.rs.template", MIGRATION_TEST),
    ("models/model.rs.template", MODEL),
    ("models/migration.rs.template", MODEL_MIGRATION),
    ("models/model_test.rs.template", MODEL_TEST),
    (
        "scaffold/controller_html.rs.template",
        SCAFFOLD_CONTROLLER_HTML,
    ),
    (
        "scaffold/controller_api.rs.template",
        SCAFFOLD_CONTROLLER_API,
    ),
    (
        "scaffold/controller_test.rs.template",
        SCAFFOLD_CONTROLLER_TEST,
    ),
    ("scaffold/views/index.html.tera", VIEW_INDEX),
    ("scaffold/views/show.html.tera", VIEW_SHOW),
    ("scaffold/views/new.html.tera", VIEW_NEW),
    ("scaffold/views/edit.html.tera", VIEW_EDIT),
    ("scaffold/views/_form.html.tera", VIEW_FORM),
];

/// Directory (relative to the project root / cwd) holding override templates.
pub fn project_root() -> PathBuf {
    PathBuf::from("templates")
}

/// All overridable built-in templates as `(rel_path, default_content)`.
pub fn builtin_templates() -> &'static [(&'static str, &'static str)] {
    BUILTIN
}

/// The built-in default for `rel`. Panics if `rel` is not a known template
/// (a programmer error — generators only request paths listed in [`BUILTIN`]).
fn builtin_default(rel: &str) -> &'static str {
    BUILTIN
        .iter()
        .find(|(p, _)| *p == rel)
        .map(|(_, c)| *c)
        .unwrap_or_else(|| panic!("unknown built-in template: {rel}"))
}

/// Read `root/rel` if it exists (logging that an override is in use), else
/// return `default`.
pub fn resolve_with_root(root: &Path, rel: &str, default: &str) -> String {
    let path = root.join(rel);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            doido_core::tracing::info!("using template override: {}", path.display());
            content
        }
        Err(_) => default.to_string(),
    }
}

/// Resolve a built-in template `rel`, preferring a project override under
/// `templates/`.
pub fn get(rel: &str) -> String {
    resolve_with_root(&project_root(), rel, builtin_default(rel))
}

/// Like [`get`] but with an explicit override root (for tests).
pub fn get_with_root(root: &Path, rel: &str) -> String {
    resolve_with_root(root, rel, builtin_default(rel))
}
