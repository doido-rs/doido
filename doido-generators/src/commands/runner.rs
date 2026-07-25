//! `doido runner <args>` — run application code. Rust can't evaluate snippets,
//! so this runs the app's own binary (`cargo run -- <args>`), letting the app
//! dispatch on the arguments.

use std::process::Command;

/// The program + args used to run application code.
pub fn runner_command(args: &[&str]) -> (String, Vec<String>) {
    let mut cargo_args = vec!["run".to_string(), "--quiet".to_string(), "--".to_string()];
    cargo_args.extend(args.iter().map(|s| s.to_string()));
    ("cargo".to_string(), cargo_args)
}

pub fn run(args: &[&str]) {
    let (program, cargo_args) = runner_command(args);
    let _ = Command::new(program).args(cargo_args).status();
}
