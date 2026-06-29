//! Runtime processing of project-local custom generators.
//!
//! A project generator is a folder of template files under
//! `lib/generators/<name>/`. Running it substitutes the name argument into each
//! file's relative path and contents, strips a trailing `.template`, and emits
//! the result. No compilation required — the project owns its generators.

use crate::generator::GeneratedFile;
use crate::generators::{to_pascal, to_snake, to_table_name};
use doido_core::{anyhow::anyhow, Result};
use std::path::{Path, PathBuf};

/// Root holding project-local custom generators.
pub fn generators_root() -> PathBuf {
    PathBuf::from("lib/generators")
}

/// The directory of the project generator named `name`, if it exists.
pub fn find(name: &str) -> Option<PathBuf> {
    find_in(&generators_root(), name)
}

/// Like [`find`] but rooted at an explicit directory (for tests).
pub fn find_in(root: &Path, name: &str) -> Option<PathBuf> {
    let dir = root.join(name);
    dir.is_dir().then_some(dir)
}

/// Names of the project-local generators discovered under `lib/generators/`.
pub fn list() -> Vec<String> {
    list_in(&generators_root())
}

/// Like [`list`] but rooted at an explicit directory (for tests).
pub fn list_in(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = match std::fs::read_dir(root) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

/// Tokens derived from a single name argument, substituted into template paths
/// and contents.
struct Tokens {
    name: String,
    snake: String,
    pascal: String,
    plural: String,
    controller: String,
}

impl Tokens {
    fn from_name(name: &str) -> Self {
        let pascal = to_pascal(name);
        let plural = to_table_name(name);
        Self {
            name: name.to_string(),
            snake: to_snake(name),
            controller: format!("{}Controller", to_pascal(&plural)),
            plural,
            pascal,
        }
    }

    fn apply(&self, s: &str) -> String {
        s.replace("{name}", &self.name)
            .replace("{snake}", &self.snake)
            .replace("{singular}", &self.snake)
            .replace("{pascal}", &self.pascal)
            .replace("{Model}", &self.pascal)
            .replace("{plural}", &self.plural)
            .replace("{Controller}", &self.controller)
    }
}

/// Process the generator at `dir` into output files. `args[0]` is the name the
/// generated artifacts are built around.
pub fn run(dir: &Path, args: &[&str]) -> Result<Vec<GeneratedFile>> {
    let name = args
        .first()
        .copied()
        .ok_or_else(|| anyhow!("this generator requires a name argument"))?;
    let tokens = Tokens::from_name(name);
    let mut files = Vec::new();
    collect(dir, dir, &tokens, &mut files)?;
    if files.is_empty() {
        return Err(anyhow!(
            "generator at {} has no template files",
            dir.display()
        ));
    }
    Ok(files)
}

/// Recursively gather template files under `dir`, skipping dotfiles and README.
fn collect(base: &Path, dir: &Path, tokens: &Tokens, out: &mut Vec<GeneratedFile>) -> Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| anyhow!("reading {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();

    for path in entries {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if file_name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect(base, &path, tokens, out)?;
        } else {
            if file_name.eq_ignore_ascii_case("README.md") {
                continue;
            }
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let out_path = strip_template_ext(&tokens.apply(&rel));
            let content = std::fs::read_to_string(&path)
                .map_err(|e| anyhow!("reading {}: {e}", path.display()))?;
            out.push(GeneratedFile {
                path: out_path,
                content: tokens.apply(&content),
            });
        }
    }
    Ok(())
}

fn strip_template_ext(p: &str) -> String {
    p.strip_suffix(".template")
        .map(str::to_string)
        .unwrap_or_else(|| p.to_string())
}
