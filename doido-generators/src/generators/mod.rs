pub mod bootstrap_migrations;
pub mod channel;
pub mod controller;
pub mod field;
pub mod generator_gen;
pub mod job;
pub mod locale;
pub mod mailer;
pub mod migration;
pub mod migration_support;
pub mod model;
pub mod new;
pub mod resource;
pub mod scaffold;
pub mod storage_adapter;
pub mod storage_install;
pub mod templates_gen;

use doido_core::Inflector;

/// `BlogPost`/`blog-post` → `blog_post`.
pub fn to_snake(s: &str) -> String {
    Inflector::underscore(s)
}

/// Insert `pub mod <module>;` into a directory `mod.rs`, just above `marker`
/// (appending if the marker is absent). Idempotent: an already-registered module
/// leaves the file unchanged. Shared by the job/mailer/channel generators.
pub(crate) fn register_module(existing: &str, module: &str, marker: &str) -> String {
    let decl = format!("pub mod {module};");
    if existing.lines().any(|l| l.trim() == decl) {
        return existing.to_string();
    }
    let mut lines: Vec<String> = existing.lines().map(String::from).collect();
    match lines.iter().position(|l| l.contains(marker)) {
        Some(i) => lines.insert(i, decl),
        None => lines.push(decl),
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// `blog_post`/`blog-post` → `BlogPost`.
pub fn to_pascal(s: &str) -> String {
    // Normalise dashes/casing first so `camelize` (which splits on `_`) sees
    // clean snake_case input.
    Inflector::camelize(&Inflector::underscore(s))
}

/// `BlogPost`/`blog_post` → `blog_posts` — the pluralized, snake_cased table
/// name, honouring any custom rules from `config/inflection.yaml`.
pub fn to_table_name(s: &str) -> String {
    Inflector::tableize(s)
}
