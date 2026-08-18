//! `doido console` — an interactive REPL.
//!
//! Rust has no built-in evaluator, so the console launches `evcxr` (the Rust
//! REPL) when it is on `PATH`, and otherwise prints a banner explaining how to
//! start one against the app's crates.

use std::process::Command;

/// The console banner (current environment + guidance).
pub fn banner() -> String {
    let env = std::env::var("DOIDO_ENV").unwrap_or_else(|_| "development".to_string());
    format!(
        "doido console ({env})\n\
         Launching the Rust REPL (evcxr) if available; otherwise install it with\n\
         `cargo install evcxr_repl`, or use `doido runner` to execute app code."
    )
}

pub fn run() {
    println!("{}", banner());
    // If evcxr is installed, drop into it; ignore failures (banner already shown).
    let _ = Command::new("evcxr").status();
}
