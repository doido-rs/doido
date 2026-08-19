use crate::auth_registry;
use crate::commands::write_files;
use crate::project_auth;
use crate::{default_registry, project_generator, Generator, GeneratorRegistry};
use doido_core::Result;
use std::path::Path;

const CARGO_TOML: &str = "Cargo.toml";

/// Build the effective registry for `base/Cargo.toml` (typically the process cwd).
pub fn registry_for_project_at(base: &Path) -> GeneratorRegistry {
    let mut reg = default_registry();
    if project_auth::project_has_doido_auth(base.join(CARGO_TOML)) {
        auth_registry::register_auth_generators(&mut reg);
    }
    reg
}

/// Build the effective registry for `base`, then merge app-supplied `extra`
/// generators on top. This is how a `doido::Doido` builder installs custom
/// generators; later registrations win on name collisions.
pub fn registry_for_project_at_with(
    base: &Path,
    extra: Vec<Box<dyn Generator>>,
) -> GeneratorRegistry {
    let mut reg = registry_for_project_at(base);
    for generator in extra {
        reg.register(generator);
    }
    reg
}

/// Build the effective registry for the current working directory.
pub fn registry_for_project() -> GeneratorRegistry {
    registry_for_project_at(Path::new("."))
}

pub fn project_has_doido_auth_at(base: &Path) -> bool {
    project_auth::project_has_doido_auth(base.join(CARGO_TOML))
}

pub fn project_has_doido_auth() -> bool {
    project_has_doido_auth_at(Path::new("."))
}

/// Entry point for `doido generate [name] [args...]`. With no name — or a help
/// flag — it lists the available generators; otherwise it runs the named one.
pub fn run(args: &[String]) {
    run_with(args, Vec::new());
}

/// Like [`run`], but with app-supplied `extra` generators merged into the
/// registry — the mechanism behind `doido::Doido::register_generator`. Custom
/// generators are listed and dispatched exactly like the framework built-ins.
pub fn run_with(args: &[String], extra: Vec<Box<dyn Generator>>) {
    let custom_names: Vec<String> = extra.iter().map(|g| g.name().to_string()).collect();
    let registry = registry_for_project_at_with(Path::new("."), extra);

    let first = args.first().map(String::as_str);
    if matches!(first, None | Some("-h" | "--help" | "help")) {
        print_generator_list(&registry, &custom_names);
        return;
    }
    let generator = args[0].as_str();
    let rest: Vec<&str> = args[1..].iter().map(String::as_str).collect();
    run_generate_with(&registry, generator, &rest);
}

/// Print the built-in, app-installed and project-local generators, to stdout
/// (the command's primary output).
fn print_generator_list(registry: &GeneratorRegistry, custom_names: &[String]) {
    let auth_installed = project_has_doido_auth();

    println!("Available generators:\n");
    println!("Built-in:");
    for name in registry.list() {
        if auth_installed && auth_registry::auth_generator_names().contains(&name) {
            continue;
        }
        if custom_names.iter().any(|c| c == name) {
            continue;
        }
        println!("  {name}");
    }

    if auth_installed {
        println!("\nAuth (doido-auth):");
        for name in auth_registry::auth_generator_names() {
            println!("  {name}");
        }
    }

    if !custom_names.is_empty() {
        println!("\nInstalled (app):");
        for name in custom_names {
            println!("  {name}");
        }
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

/// Run a generator by name against the process-cwd registry. Used by internal
/// callers (e.g. `doido new --auth`) that don't install extra generators.
pub fn run_generate(generator: &str, args: &[&str]) {
    run_generate_with(&registry_for_project(), generator, args);
}

fn run_generate_with(registry: &GeneratorRegistry, generator: &str, args: &[&str]) {
    match resolve_and_run(registry, generator, args) {
        Ok(files) => {
            if files.is_empty() {
                doido_core::tracing::info!("no files generated");
                return;
            }
            if let Err(e) = write_files(&files, Path::new(".")) {
                doido_core::tracing::error!("error writing files: {e}");
                std::process::exit(1);
            }
            crate::commands::sync_model_extensions_at(Path::new("."));
        }
        Err(e) => {
            doido_core::tracing::error!("{e}");
            std::process::exit(1);
        }
    }
}

/// Run a generator from `registry`, or fall back to a project-local generator
/// under `lib/generators/<name>/`.
fn resolve_and_run(
    registry: &GeneratorRegistry,
    generator: &str,
    args: &[&str],
) -> Result<Vec<crate::GeneratedFile>> {
    if auth_registry::auth_generator_names().contains(&generator) && !project_has_doido_auth() {
        return Err(doido_core::anyhow::anyhow!(
            "auth generator '{generator}' requires doido-auth in Cargo.toml. \
             Add the dependency (e.g. `cargo add doido-auth`) or scaffold with `doido new --auth`."
        ));
    }

    if registry.list().contains(&generator) {
        registry.run(generator, args)
    } else if let Some(dir) = project_generator::find(generator) {
        doido_core::tracing::info!("using project generator: {}", dir.display());
        project_generator::run(&dir, args)
    } else {
        let mut hint = format!(
            "unknown generator '{generator}'. built-in: {}",
            registry.list().join(", ")
        );
        if !project_has_doido_auth() {
            hint.push_str(". auth generators require doido-auth in Cargo.toml");
        }
        Err(doido_core::anyhow::anyhow!(hint))
    }
}
