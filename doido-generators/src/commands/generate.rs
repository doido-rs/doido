use crate::commands::write_files;
use crate::{default_registry, project_generator};
use doido_core::Result;
use std::path::Path;

/// Entry point for `doido generate [name] [args...]`. With no name — or a help
/// flag — it lists the available generators; otherwise it runs the named one.
pub fn run(args: &[String]) {
    let first = args.first().map(String::as_str);
    if matches!(first, None | Some("-h" | "--help" | "help")) {
        print_generator_list();
        return;
    }
    let generator = args[0].as_str();
    let rest: Vec<&str> = args[1..].iter().map(String::as_str).collect();
    run_generate(generator, &rest);
}

/// Print the built-in and project-local generators, to stdout (the command's
/// primary output).
fn print_generator_list() {
    let registry = default_registry();
    println!("Available generators:\n");
    println!("Built-in:");
    for name in registry.list() {
        println!("  {name}");
    }

    let project = project_generator::list();
    if !project.is_empty() {
        println!("\nProject (lib/generators/):");
        for name in project {
            println!("  {name}");
        }
    }

    println!("\nUsage: doido generate <name> [args...]");
}

pub fn run_generate(generator: &str, args: &[&str]) {
    match resolve_and_run(generator, args) {
        Ok(files) => {
            if files.is_empty() {
                doido_core::tracing::info!("no files generated");
                return;
            }
            if let Err(e) = write_files(&files, Path::new(".")) {
                doido_core::tracing::error!("error writing files: {e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            doido_core::tracing::error!("{e}");
            std::process::exit(1);
        }
    }
}

/// Run a built-in generator, or fall back to a project-local generator under
/// `lib/generators/<name>/`.
fn resolve_and_run(generator: &str, args: &[&str]) -> Result<Vec<crate::GeneratedFile>> {
    let registry = default_registry();
    if registry.list().contains(&generator) {
        registry.run(generator, args)
    } else if let Some(dir) = project_generator::find(generator) {
        doido_core::tracing::info!("using project generator: {}", dir.display());
        project_generator::run(&dir, args)
    } else {
        Err(doido_core::anyhow::anyhow!(
            "unknown generator '{generator}'. built-in: {}. project generators live in lib/generators/<name>/",
            registry.list().join(", ")
        ))
    }
}
