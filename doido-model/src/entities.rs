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

/// Default extension stub for a model module — re-exports the generated entity.
pub fn extension_stub(module: &str) -> String {
    format!(
        "//! Model extensions for `{module}` — safe to edit; never overwritten by generators.\n\
         //!\n\
         //! The SeaORM entity definition lives in `_entities/{module}.rs` and is\n\
         //! regenerated on every `doido db migrate`.\n\n\
         pub use super::_entities::{module}::*;\n"
    )
}

/// After entity export, ensure each entity has a non-destructive extension stub in
/// `app/models/<name>.rs` and is registered in `app/models/mod.rs`.
pub fn sync_extension_stubs(entities_dir: &Path, models_dir: &Path) -> Result<()> {
    let entities_mod_path = entities_dir.join("mod.rs");
    let entities_mod = fs::read_to_string(&entities_mod_path)
        .map_err(|e| doido_core::anyhow::anyhow!("read {}: {e}", entities_mod_path.display()))?;

    let models_mod_path = models_dir.join("mod.rs");
    let mut models_mod = fs::read_to_string(&models_mod_path)
        .map_err(|e| doido_core::anyhow::anyhow!("read {}: {e}", models_mod_path.display()))?;

    for module in entity_modules(&entities_mod) {
        let extension_path = models_dir.join(format!("{module}.rs"));
        if !extension_path.is_file() {
            fs::write(&extension_path, extension_stub(&module))?;
        }
        models_mod = register_model_module(&models_mod, &module);
    }

    fs::write(&models_mod_path, models_mod)?;
    Ok(())
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
