//! Startup banner: a solid-block FIGlet "DOIDO" logo followed by a Loco-style
//! runtime info block. Printed to stderr so it never interferes with stdout
//! output (route tables, entity codegen, etc.). Colorized only when stderr is a
//! TTY.

use figlet_rs::FIGlet;
use std::io::{IsTerminal, Write};

/// Backend name from a connection URL (the scheme before `://`).
fn db_backend(url: &str) -> &str {
    match url.split_once("://") {
        Some((scheme, _)) => scheme,
        None => "unknown",
    }
}

/// Print the startup banner to stderr. `mode` is the running subcommand
/// (server, worker, console, …) shown on the `modes:` line.
pub fn print(mode: &str) {
    let mut out = std::io::stderr();
    let color = out.is_terminal();
    let green = if color { "\x1b[1;32m" } else { "" };
    let dim = if color { "\x1b[2m" } else { "" };
    let reset = if color { "\x1b[0m" } else { "" };

    if let Ok(font) = FIGlet::standard() {
        if let Some(figure) = font.convert("DOIDO") {
            let _ = writeln!(out);
            for line in figure.to_string().lines() {
                let _ = writeln!(out, "{green}{line}{reset}");
            }
        }
    }

    let _ = writeln!(
        out,
        "{dim}      doido · rails-inspired rust framework · v{}{reset}",
        env!("CARGO_PKG_VERSION"),
    );
    let _ = writeln!(out);

    let environment = doido_core::Environment::get_env().to_string();
    let database = doido_model::config::load().database().url.clone();
    let logger = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let compilation = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    let _ = writeln!(out, "{:>11}: {}", "environment", environment);
    let _ = writeln!(out, "{:>11}: {}", "database", db_backend(&database));
    let _ = writeln!(out, "{:>11}: {}", "logger", logger);
    let _ = writeln!(out, "{:>11}: {}", "compilation", compilation);
    let _ = writeln!(out, "{:>11}: {}", "modes", mode);
    let _ = writeln!(out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_backend_extracts_scheme_before_the_separator() {
        assert_eq!(db_backend("postgres://user@host/db"), "postgres");
        assert_eq!(db_backend("sqlite://db/dev.db"), "sqlite");
        // A URL without `://` has no scheme to extract.
        assert_eq!(db_backend("not-a-url"), "unknown");
    }

    #[test]
    fn print_writes_the_banner_without_panicking() {
        // Exercises the full info block (stderr is not a TTY under test, so the
        // no-color branches are taken). Config falls back to defaults.
        print("server");
        print("worker");
    }
}
