//! Entity export layout — `_entities/` holds regenerated SeaORM definitions;
//! `app/models/<name>.rs` holds safe-to-edit extensions.

use doido_core::Result;
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
         pub use super::_entities::{entity_module}::*;\n"
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
            let imports_fixed = rewrite_entity_file_imports(&content);
            ensure_inner_attribute(&imports_fixed, "#![allow(dead_code, unused_imports)]")
        };
        if rewritten != content {
            fs::write(path, rewritten)?;
        }
    }
    Ok(())
}

fn rewrite_entity_file_imports(content: &str) -> String {
    if content.contains("use doido::model::sea_orm as sea_orm") {
        return content.to_string();
    }

    let mut prelude_found = false;
    let mut extra_imports: Vec<String> = Vec::new();
    let mut other_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "use sea_orm::entity::prelude::*;" {
            prelude_found = true;
        } else if let Some(rest) = trimmed.strip_prefix("use sea_orm::") {
            extra_imports.push(format!("use doido::model::sea_orm::{rest}"));
        } else {
            other_lines.push(line.to_string());
        }
    }

    if !prelude_found && extra_imports.is_empty() && !content.contains("#[sea_orm") {
        return content.to_string();
    }

    let mut insert_at = 0;
    for (i, line) in other_lines.iter().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//!") || t.starts_with("//") {
            insert_at = i + 1;
        } else if t.starts_with("use ") {
            insert_at = i;
            break;
        } else {
            break;
        }
    }

    let mut imports = vec!["use doido::model::sea_orm as sea_orm;".to_string()];
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

fn ensure_inner_attribute(content: &str, attr: &str) -> String {
    if content.contains(attr) {
        return content.to_string();
    }

    let mut insert_at = 0;
    for (i, line) in content.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//!") || t.starts_with("//") {
            insert_at = i + 1;
        } else if t.starts_with("#![") {
            return content.to_string();
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

/// Post-processes exported entities so they compile inside a Doido app.
pub fn postprocess_entity_export(entities_dir: &Path, _models_dir: &Path) -> Result<()> {
    rewrite_generated_imports(entities_dir)
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
