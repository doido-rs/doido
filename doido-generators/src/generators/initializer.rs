use crate::generator::{GeneratedFile, Generator};
use crate::generators::to_snake;
use doido_core::Result;

/// Fallback `config/initializers/mod.rs` when the app doesn't have one on disk yet.
const INITIALIZERS_MOD_BASE: &str = include_str!("../../templates/new/config/initializers/mod.rs");
const INITIALIZERS_MOD_PATH: &str = "config/initializers/mod.rs";

pub struct InitializerGenerator;

pub(crate) fn render_initializer_content(snake: &str) -> String {
    crate::templates::get("initializer/initializer.rs.template").replace("{snake}", snake)
}

pub(crate) fn render_initializer_test(snake: &str) -> String {
    crate::templates::get("initializer/initializer_test.rs.template").replace("{snake}", snake)
}

pub(crate) fn read_initializers_mod() -> String {
    std::fs::read_to_string(INITIALIZERS_MOD_PATH)
        .unwrap_or_else(|_| INITIALIZERS_MOD_BASE.to_string())
}

pub fn register_initializer_in_mod(existing: &str, snake: &str) -> String {
    let mod_decl = format!("pub mod {snake};");
    let register_decl = format!("    inits.register(\"{snake}\", {snake}::init);");

    let mut lines: Vec<String> = existing.lines().map(String::from).collect();

    // 1. Insert module declaration if not already present
    if !lines.iter().any(|l| l.trim() == mod_decl) {
        if let Some(pos) = lines.iter().position(|l| l.contains("@generated-modules")) {
            lines.insert(pos, mod_decl);
        } else {
            lines.insert(0, mod_decl);
        }
    }

    // 2. Insert inits.register(...) if not already present
    if !lines.iter().any(|l| l.trim() == register_decl.trim()) {
        if let Some(pos) = lines
            .iter()
            .position(|l| l.contains("@generated-initializers"))
        {
            lines.insert(pos, register_decl);
        } else if let Some(pos) = lines
            .iter()
            .position(|l| l.contains("register_initializers"))
        {
            lines.insert(pos + 1, register_decl);
        } else {
            lines.push(register_decl);
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

impl Generator for InitializerGenerator {
    fn name(&self) -> &str {
        "initializer"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("initializer generator requires a name argument")
        })?;
        let snake = to_snake(name);
        let content = render_initializer_content(&snake);
        let test = render_initializer_test(&snake);
        let initializers_mod = register_initializer_in_mod(&read_initializers_mod(), &snake);

        Ok(vec![
            GeneratedFile {
                path: format!("config/initializers/{snake}.rs"),
                content,
            },
            GeneratedFile {
                path: INITIALIZERS_MOD_PATH.to_string(),
                content: initializers_mod,
            },
            GeneratedFile {
                path: format!("tests/{snake}_initializer_test.rs"),
                content: test,
            },
        ])
    }
}
