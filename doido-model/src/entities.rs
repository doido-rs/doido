//! Entity export layout — `_entities/` holds regenerated SeaORM definitions;
//! `app/models/<name>.rs` holds safe-to-edit extensions.

use doido_core::{Inflector, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Where `doido db migrate` and `doido db generate entity` write SeaORM entities.
pub const DEFAULT_ENTITY_DIR: &str = "app/models/_entities";

const MODELS_MOD_MARKER: &str = "@generated-models";
const ENTITIES_MOD_MARKER: &str = "@generated-entities";

/// Module names declared in an `_entities/mod.rs` (or `lib.rs`) index file.
pub fn entity_modules(mod_content: &str) -> Vec<String> {
    mod_content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("pub mod ")?;
            let name = rest.trim_end_matches(';').trim();
            if name.is_empty() || matches!(name, "prelude" | "sea_orm_active_enums" | "lib") {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

/// Model modules registered in `app/models/mod.rs` (excluding `_entities`).
pub fn model_modules(models_mod: &str) -> Vec<String> {
    models_mod
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("pub mod ")?;
            let name = rest.trim_end_matches(';').trim();
            if name.is_empty() || name == "_entities" {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

/// Default extension stub — re-exports the generated entity module (table name).
pub fn extension_stub(model_module: &str, entity_module: &str) -> String {
    format!(
        "//! Model extensions for `{model_module}` — safe to edit; never overwritten by generators.\n\
         //!\n\
         //! The SeaORM entity definition lives in `_entities/{entity_module}.rs` and is\n\
         //! regenerated on every `doido db migrate`.\n\
         #![allow(dead_code, unused_imports)]\n\n\
         pub use super::_entities::{entity_module}::*;\n\n\
         use doido::model::sea_orm::ActiveModelBehavior;\n\n\
         impl ActiveModelBehavior for ActiveModel {{}}\n"
    )
}

/// Rewrites SeaORM CLI imports to the mandatory `doido::model::sea_orm` path
/// and adds lint allows so exported entities compile under `-D warnings`.
pub fn rewrite_generated_imports(entities_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(entities_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == "mod.rs" || name == "lib.rs" {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        let rewritten = if name == "prelude.rs" {
            ensure_inner_attribute(&content, "#![allow(unused_imports)]")
        } else {
            let with_allow =
                ensure_inner_attribute(&content, "#![allow(dead_code, unused_imports)]");
            let imports_fixed = rewrite_entity_file_imports(&with_allow);
            strip_active_model_behavior(&imports_fixed)
        };
        if rewritten != content {
            fs::write(path, rewritten)?;
        }
    }
    Ok(())
}

fn rewrite_entity_file_imports(content: &str) -> String {
    let mut prelude_found = false;
    let mut extra_imports: Vec<String> = Vec::new();
    let mut other_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "use sea_orm::entity::prelude::*;"
            || trimmed == "use doido::model::sea_orm::entity::prelude::*;"
        {
            prelude_found = true;
        } else if trimmed == "use doido::model::sea_orm;"
            || trimmed == "use doido::model::sea_orm as sea_orm;"
        {
            // Normalized below — drop legacy single-path imports.
        } else if let Some(rest) = trimmed.strip_prefix("use sea_orm::") {
            extra_imports.push(format!("use doido::model::sea_orm::{rest}"));
        } else {
            other_lines.push(line.to_string());
        }
    }

    if !prelude_found
        && extra_imports.is_empty()
        && !content.contains("#[sea_orm")
        && !content.contains("use doido::model::sea_orm")
    {
        return content.to_string();
    }

    let mut insert_at = 0;
    for (i, line) in other_lines.iter().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//!") || t.starts_with("//") || t.starts_with("#![") {
            insert_at = i + 1;
        } else if t.starts_with("use ") {
            insert_at = i;
            break;
        } else {
            break;
        }
    }

    let mut imports = vec!["use doido::model::sea_orm;".to_string()];
    if prelude_found || content.contains("#[sea_orm") {
        imports.push("use doido::model::sea_orm::entity::prelude::*;".to_string());
    }
    imports.extend(extra_imports);

    other_lines.splice(insert_at..insert_at, imports);
    let mut out = other_lines.join("\n");
    if content.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Removes the default SeaORM `ActiveModelBehavior` impl from exported entity files.
/// The impl belongs in `app/models/<name>.rs` extension stubs instead.
fn strip_active_model_behavior(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    lines.retain(|line| line.trim() != "impl ActiveModelBehavior for ActiveModel {}");
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    let mut out = lines.join("\n");
    if content.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn ensure_inner_attribute(content: &str, attr: &str) -> String {
    if content.contains(attr) {
        return content.to_string();
    }
    if attr == "#![allow(dead_code, unused_imports)]" && content.contains("#![allow(dead_code)]") {
        return content.replace("#![allow(dead_code)]", attr);
    }

    let mut insert_at = 0;
    for (i, line) in content.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//!") || t.starts_with("//") || t.starts_with("#![") {
            insert_at = i + 1;
        } else {
            break;
        }
    }

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    lines.insert(insert_at, attr.to_string());
    if insert_at == lines.len().saturating_sub(1)
        || lines.get(insert_at + 1).is_none_or(|l| !l.is_empty())
    {
        lines.insert(insert_at + 1, String::new());
    }
    let mut out = lines.join("\n");
    if content.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Returns the `_entities/<name>` module re-exported by a model extension, if any.
pub fn reexported_entity_module(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("pub use super::_entities::") else {
            continue;
        };
        let Some(entity) = rest.strip_suffix("::*;") else {
            continue;
        };
        let entity = entity.trim();
        if !entity.is_empty() {
            return Some(entity.to_string());
        }
    }
    None
}

/// Returns true when some model extension already re-exports `entity_module`.
pub fn entity_has_model_extension(models_dir: &Path, entity_module: &str) -> bool {
    fs::read_dir(models_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension() == Some(std::ffi::OsStr::new("rs"))
                && entry.file_name() != "mod.rs"
        })
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .any(|content| reexported_entity_module(&content).as_deref() == Some(entity_module))
}

/// Removes duplicate model extensions that re-export the same entity module.
/// Keeps the canonical stub (prefer a module name different from the table name,
/// e.g. `sku` over `skus` for entity `skus`).
pub fn dedupe_model_extension_stubs(models_dir: &Path) -> Result<()> {
    let models_mod_path = models_dir.join("mod.rs");
    let models_mod = fs::read_to_string(&models_mod_path).unwrap_or_default();

    let mut by_entity: HashMap<String, Vec<String>> = HashMap::new();
    for entry in fs::read_dir(models_dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == "mod" {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        if let Some(entity) = reexported_entity_module(&content) {
            by_entity.entry(entity).or_default().push(stem.to_string());
        }
    }

    let mut models_mod_updated = models_mod.clone();
    for (entity, mut models) in by_entity {
        if models.len() <= 1 {
            continue;
        }
        models.sort();
        let keep = models
            .iter()
            .find(|name| **name != entity)
            .or(models.first())
            .expect("len > 1")
            .clone();
        for duplicate in &models {
            if duplicate == &keep {
                continue;
            }
            fs::remove_file(models_dir.join(format!("{duplicate}.rs")))?;
            let decl = format!("pub mod {duplicate};");
            models_mod_updated = models_mod_updated
                .lines()
                .filter(|line| line.trim() != decl)
                .collect::<Vec<_>>()
                .join("\n");
        }
    }

    if models_mod_updated != models_mod {
        let mut out = models_mod_updated;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        fs::write(&models_mod_path, out)?;
    }

    Ok(())
}

/// Creates missing `app/models/<name>.rs` extension stubs (with
/// `ActiveModelBehavior`) for every entity module under `_entities/`.
pub fn ensure_model_extension_stubs(entities_dir: &Path, models_dir: &Path) -> Result<()> {
    let entities_mod_path = entities_dir.join("mod.rs");
    let models_mod_path = models_dir.join("mod.rs");

    let entities_mod = fs::read_to_string(&entities_mod_path).unwrap_or_default();
    let models_mod = fs::read_to_string(&models_mod_path).unwrap_or_default();

    let entity_modules = entity_modules(&entities_mod);
    let existing_models: HashSet<String> = model_modules(&models_mod).into_iter().collect();

    let mut models_mod_updated = models_mod.clone();

    for entity_module in entity_modules {
        if entity_has_model_extension(models_dir, &entity_module) {
            continue;
        }
        let model_module = Inflector::singularize(&entity_module);
        let model_path = models_dir.join(format!("{model_module}.rs"));
        if model_path.exists() || existing_models.contains(&model_module) {
            continue;
        }
        fs::write(&model_path, extension_stub(&model_module, &entity_module))?;
        models_mod_updated = register_model_module(&models_mod_updated, &model_module);
    }

    if models_mod_updated != models_mod {
        fs::write(&models_mod_path, models_mod_updated)?;
    }

    Ok(())
}

const ACTIVE_MODEL_BEHAVIOR_IMPL: &str = "impl ActiveModelBehavior for ActiveModel {}";
const ACTIVE_MODEL_BEHAVIOR_MARKER: &str = "impl ActiveModelBehavior for ActiveModel";

/// Returns true when a model extension already defines `ActiveModelBehavior`
/// for the re-exported `ActiveModel` (empty stub or a full custom impl).
fn extension_has_active_model_behavior(content: &str) -> bool {
    content
        .split(ACTIVE_MODEL_BEHAVIOR_MARKER)
        .nth(1)
        .is_some_and(|rest| rest.trim_start().starts_with('{'))
}

/// Returns true when a model extension re-exports an entity and provides
/// `ActiveModelBehavior` for its `ActiveModel`.
pub fn model_extension_covers_entity(content: &str, entity_module: &str) -> bool {
    content.contains(&format!("pub use super::_entities::{entity_module}::*"))
        && extension_has_active_model_behavior(content)
}

/// Inserts the default `ActiveModelBehavior` impl into model extensions that
/// re-export an entity but predate the generator template update.
pub fn ensure_active_model_behavior_in_extensions(models_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(models_dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        if path.file_name().is_some_and(|name| name == "mod.rs") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        if !content.contains("pub use super::_entities::") {
            continue;
        }
        if extension_has_active_model_behavior(&content) {
            continue;
        }
        let updated = inject_active_model_behavior(&content);
        if updated != content {
            fs::write(path, updated)?;
        }
    }
    Ok(())
}

fn inject_active_model_behavior(content: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut insert_at = None;
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("pub use super::_entities::") {
            insert_at = Some(i + 1);
            break;
        }
    }
    let Some(at) = insert_at else {
        return content.to_string();
    };
    lines.insert(at, String::new());
    lines.insert(
        at + 1,
        "use doido::model::sea_orm::ActiveModelBehavior;".to_string(),
    );
    lines.insert(at + 2, String::new());
    lines.insert(at + 3, ACTIVE_MODEL_BEHAVIOR_IMPL.to_string());
    let mut out = lines.join("\n");
    if content.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn uncovered_entity_modules(entities_dir: &Path, models_dir: &Path) -> Result<Vec<String>> {
    let entities_mod = fs::read_to_string(entities_dir.join("mod.rs")).unwrap_or_default();
    let modules = entity_modules(&entities_mod);

    let mut model_contents = Vec::new();
    for entry in fs::read_dir(models_dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        if path.file_name().is_some_and(|name| name == "mod.rs") {
            continue;
        }
        model_contents.push(fs::read_to_string(path)?);
    }

    Ok(modules
        .into_iter()
        .filter(|entity| {
            !model_contents
                .iter()
                .any(|content| model_extension_covers_entity(content, entity))
        })
        .collect())
}

/// Writes `_entities/active_model_behavior.rs` with fallback impls for entities
/// that have no covering model extension (e.g. inline tutorial models).
pub fn write_active_model_behavior_module(entities_dir: &Path, models_dir: &Path) -> Result<()> {
    let uncovered = uncovered_entity_modules(entities_dir, models_dir)?;
    let behavior_path = entities_dir.join("active_model_behavior.rs");
    let entities_mod_path = entities_dir.join("mod.rs");
    let entities_mod = fs::read_to_string(&entities_mod_path).unwrap_or_default();

    if uncovered.is_empty() {
        if behavior_path.exists() {
            fs::remove_file(behavior_path)?;
        }
        let without = entities_mod
            .lines()
            .filter(|line| line.trim() != "pub mod active_model_behavior;")
            .collect::<Vec<_>>()
            .join("\n");
        let mut cleaned = without;
        if entities_mod.ends_with('\n') {
            cleaned.push('\n');
        }
        if cleaned != entities_mod {
            fs::write(entities_mod_path, cleaned)?;
        }
        return Ok(());
    }

    let mut body = String::from(
        "//! Default ActiveModelBehavior for entities without a covering model extension.\n\
         //! Regenerated on `doido db migrate` — do not edit.\n\
         #![allow(dead_code)]\n\n\
         use doido::model::sea_orm::ActiveModelBehavior;\n\n",
    );
    for entity in &uncovered {
        body.push_str(&format!(
            "impl ActiveModelBehavior for super::{entity}::ActiveModel {{}}\n"
        ));
    }
    fs::write(behavior_path, body)?;

    let updated = register_entity_module(&entities_mod, "active_model_behavior");
    if updated != entities_mod {
        fs::write(entities_mod_path, updated)?;
    }
    Ok(())
}

/// Post-processes exported entities so they compile inside a Doido app.
pub fn postprocess_entity_export(entities_dir: &Path, models_dir: &Path) -> Result<()> {
    rewrite_generated_imports(entities_dir)?;
    dedupe_model_extension_stubs(models_dir)?;
    ensure_model_extension_stubs(entities_dir, models_dir)?;
    ensure_active_model_behavior_in_extensions(models_dir)?;
    write_active_model_behavior_module(entities_dir, models_dir)
}

/// Inserts `pub mod <module>;` into `app/models/mod.rs` just above the marker.
pub fn register_model_module(models_mod: &str, module: &str) -> String {
    let decl = format!("pub mod {module};");
    if models_mod.lines().any(|l| l.trim() == decl) {
        return models_mod.to_string();
    }

    let mut lines: Vec<String> = models_mod.lines().map(String::from).collect();
    if let Some(i) = lines.iter().position(|l| l.contains(MODELS_MOD_MARKER)) {
        lines.insert(i, decl);
    } else {
        lines.push(decl);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Inserts `pub mod <module>;` into `_entities/mod.rs` just above the marker.
pub fn register_entity_module(entities_mod: &str, module: &str) -> String {
    let decl = format!("pub mod {module};");
    if entities_mod.lines().any(|l| l.trim() == decl) {
        return entities_mod.to_string();
    }

    let mut lines: Vec<String> = entities_mod.lines().map(String::from).collect();
    if let Some(i) = lines.iter().position(|l| l.contains(ENTITIES_MOD_MARKER)) {
        lines.insert(i, decl);
    } else {
        lines.push(decl);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
