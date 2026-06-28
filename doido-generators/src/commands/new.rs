use crate::commands::write_files;
use crate::default_registry;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn prompt_database() -> String {
    // Interactive prompt: written directly to stdout (not the logger) so it
    // appears inline before the user's input on the same line.
    print!("Which database? [sqlite/postgres/mysql] (default: sqlite): ");
    io::stdout().flush().expect("failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read input");
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        "sqlite".to_string()
    } else {
        trimmed
    }
}

pub fn run_new(name: &str, database: Option<&str>) {
    let db = match database {
        Some(d) => d.to_string(),
        None => prompt_database(),
    };

    match db.as_str() {
        "sqlite" | "postgres" | "mysql" => {}
        other => {
            doido_core::tracing::error!(
                "unknown database '{other}'. Use sqlite, postgres, or mysql."
            );
            std::process::exit(1);
        }
    }

    let registry = default_registry();
    let db_arg = format!("--database={db}");
    match registry.run("new", &[name, &db_arg]) {
        Ok(files) => {
            if let Err(e) = write_files(&files, Path::new(".")) {
                doido_core::tracing::error!("error writing files: {e}");
                std::process::exit(1);
            }
            let git_result = Command::new("git").args(["init", name]).output();
            match git_result {
                Ok(output) if output.status.success() => {
                    doido_core::tracing::info!("init {name}/.git");
                }
                _ => doido_core::tracing::warn!(
                    "git init failed. Run it manually: git init {name}"
                ),
            }
            doido_core::tracing::info!("created '{name}'. Next: cd {name} && cargo build");
        }
        Err(e) => {
            doido_core::tracing::error!("{e}");
            std::process::exit(1);
        }
    }
}
