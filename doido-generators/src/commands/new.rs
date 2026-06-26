use crate::commands::write_files;
use crate::default_registry;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn prompt_database() -> String {
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
            eprintln!(
                "Error: Unknown database '{}'. Use sqlite, postgres, or mysql.",
                other
            );
            std::process::exit(1);
        }
    }

    let registry = default_registry();
    let db_arg = format!("--database={db}");
    match registry.run("new", &[name, &db_arg]) {
        Ok(files) => {
            if let Err(e) = write_files(&files, Path::new(".")) {
                eprintln!("Error writing files: {e}");
                std::process::exit(1);
            }
            let git_result = Command::new("git").args(["init", name]).output();
            match git_result {
                Ok(output) if output.status.success() => {
                    println!("      init  {name}/.git");
                }
                _ => eprintln!("Warning: git init failed. Run it manually: git init {name}"),
            }
            println!("\nCreated '{name}'. Next: cd {name} && cargo build");
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
