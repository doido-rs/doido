use crate::auth_registry;
use crate::commands::write_files;
use crate::project_auth;
use crate::{default_registry, project_generator, GeneratorRegistry};
#[cfg(feature = "payments-generators")]
use crate::{payments_registry, project_payments};
use doido_core::Result;
use std::path::Path;

const CARGO_TOML: &str = "Cargo.toml";

#[cfg(feature = "payments-generators")]
fn payments_generators_enabled_at(base: &Path) -> bool {
    #[cfg(feature = "payments-generators-always")]
    {
        let _ = base;
        true
    }
    #[cfg(all(
        feature = "payments-generators",
        not(feature = "payments-generators-always")
    ))]
    {
        project_payments::project_has_doido_payments(base.join(CARGO_TOML))
    }
    #[cfg(not(feature = "payments-generators"))]
    {
        let _ = base;
        false
    }
}

/// Build the effective registry for `base/Cargo.toml` (typically the process cwd).
pub fn registry_for_project_at(base: &Path) -> GeneratorRegistry {
    let mut reg = default_registry();
    if project_auth::project_has_doido_auth(base.join(CARGO_TOML)) {
        auth_registry::register_auth_generators(&mut reg);
    }
    #[cfg(feature = "payments-generators")]
    if payments_generators_enabled_at(base) {
        payments_registry::register_payments_generators(&mut reg);
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

#[cfg(feature = "payments-generators")]
pub fn project_has_doido_payments_at(base: &Path) -> bool {
    project_payments::project_has_doido_payments(base.join(CARGO_TOML))
}

#[cfg(feature = "payments-generators")]
pub fn project_has_doido_payments() -> bool {
    project_has_doido_payments_at(Path::new("."))
}

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
    let registry = registry_for_project();
    let auth_installed = project_has_doido_auth();
    #[cfg(feature = "payments-generators")]
    let payments_installed = payments_generators_enabled_at(Path::new("."));

    println!("Available generators:\n");
    println!("Built-in:");
    for name in registry.list() {
        if auth_installed && auth_registry::auth_generator_names().contains(&name) {
            continue;
        }
        #[cfg(feature = "payments-generators")]
        if payments_installed && payments_registry::payments_generator_names().contains(&name) {
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

    #[cfg(feature = "payments-generators")]
    if payments_installed {
        println!("\nPayments (doido-payments):");
        for name in payments_registry::payments_generator_names() {
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
            crate::commands::sync_model_extensions_at(Path::new("."));
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
    if auth_registry::auth_generator_names().contains(&generator) && !project_has_doido_auth() {
        return Err(doido_core::anyhow::anyhow!(
            "auth generator '{generator}' requires doido-auth in Cargo.toml. \
             Add the dependency (e.g. `cargo add doido-auth`) or scaffold with `doido new --auth`."
        ));
    }

    #[cfg(feature = "payments-generators")]
    if payments_registry::payments_generator_names().contains(&generator)
        && !payments_generators_enabled_at(Path::new("."))
    {
        return Err(doido_core::anyhow::anyhow!(
            "payments generator '{generator}' requires doido-payments in Cargo.toml. \
             Add the dependency (e.g. `cargo add doido-payments`) or use the `doido-payments` CLI."
        ));
    }

    let registry = registry_for_project();
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
        #[cfg(feature = "payments-generators")]
        if !payments_generators_enabled_at(Path::new(".")) {
            hint.push_str(". payments generators require doido-payments in Cargo.toml");
        }
        Err(doido_core::anyhow::anyhow!(hint))
    }
}
