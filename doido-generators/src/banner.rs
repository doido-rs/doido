//! Startup banner: a solid-block FIGlet "DOIDO" logo followed by a Loco-style
//! runtime info block. Printed to stderr so it never interferes with stdout
//! output (route tables, entity codegen, etc.). Colorized only when stderr is a
//! TTY.

use figlet_rs::FIGfont;
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

    // Logo: render "DOIDO" with the standard FIGlet font, printed as-is for
    // legibility. Bail quietly if the font is unavailable.
    if let Ok(font) = FIGfont::standard() {
        if let Some(figure) = font.convert("DOIDO") {
            let _ = writeln!(out);
            for line in figure.to_string().lines() {
                let _ = writeln!(out, "{green}{line}{reset}");
            }
        }
    }

    // Tagline.
    let _ = writeln!(
        out,
        "{dim}      doido · rails-inspired rust framework · v{}{reset}",
        env!("CARGO_PKG_VERSION"),
    );
    let _ = writeln!(out);

    // Info block (values sourced honestly, never fabricated).
    let environment = doido_model::Environment::get_env().to_string();
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
