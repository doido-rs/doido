use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_pascal, to_snake};
use doido_core::Result;

/// Fallback `app/helpers/mod.rs` when the app doesn't have one on disk yet.
const HELPERS_MOD_BASE: &str = include_str!("../../templates/new/app/helpers/mod.rs");
const HELPERS_MOD_PATH: &str = "app/helpers/mod.rs";

pub struct HelperGenerator;

/// `Posts` → `PostsHelper`; `PostsHelper` stays as-is.
fn helper_type_name(name: &str) -> String {
    let pascal = to_pascal(name);
    if pascal.ends_with("Helper") {
        pascal
    } else {
        format!("{pascal}Helper")
    }
}

/// Module/file stem: `posts_helper`, not `posts_helper_helper`.
fn helper_module_snake(name: &str) -> String {
    let snake = to_snake(name);
    if snake.ends_with("_helper") {
        snake
    } else {
        format!("{snake}_helper")
    }
}

impl Generator for HelperGenerator {
    fn name(&self) -> &str {
        "helper"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("helper generator requires a name argument")
        })?;
        let snake = helper_module_snake(name);
        let pascal = helper_type_name(name);
        let content = crate::templates::get("helper/helper.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake);
        let test = crate::templates::get("helper/helper_test.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake);

        let existing = std::fs::read_to_string(HELPERS_MOD_PATH)
            .unwrap_or_else(|_| HELPERS_MOD_BASE.to_string());
        let helpers_mod = register_helper(&existing, &snake, &pascal);

        Ok(vec![
            GeneratedFile {
                path: format!("app/helpers/{snake}.rs"),
                content,
            },
            GeneratedFile {
                path: HELPERS_MOD_PATH.to_string(),
                content: helpers_mod,
            },
            GeneratedFile {
                path: format!("tests/{snake}_test.rs"),
                content: test,
            },
        ])
    }
}

/// Appends `pub mod <name>;` + `pub use …::NameHelper` to `app/helpers/mod.rs`.
/// Idempotent: skips when the module is already declared.
fn register_helper(helpers_mod: &str, snake: &str, pascal: &str) -> String {
    let decl = format!("pub mod {snake};");
    if helpers_mod.lines().any(|l| l.trim() == decl) {
        return helpers_mod.to_string();
    }
    let use_line = format!("pub use {snake}::{pascal};");
    let marker = "@generated-helpers";
    let mut lines: Vec<String> = helpers_mod.lines().map(String::from).collect();
    match lines.iter().position(|l| l.contains(marker)) {
        Some(i) => {
            lines.insert(i, use_line);
            lines.insert(i, decl);
        }
        None => {
            lines.push(decl);
            lines.push(use_line);
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
