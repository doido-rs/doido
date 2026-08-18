//! `doido dbconsole` — open the database's native client for the configured URL
//! (Rails `dbconsole`).

use std::process::Command;

/// Resolve the client program + args for a database URL, or `None` if unknown.
pub fn client_command(database_url: &str) -> Option<(String, Vec<String>)> {
    if let Some(path) = database_url
        .strip_prefix("sqlite://")
        .or_else(|| database_url.strip_prefix("sqlite:"))
    {
        Some(("sqlite3".to_string(), vec![path.to_string()]))
    } else if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        Some(("psql".to_string(), vec![database_url.to_string()]))
    } else if database_url.starts_with("mysql://") {
        Some(("mysql".to_string(), vec![database_url.to_string()]))
    } else {
        None
    }
}

/// Launch the client for `database_url`.
pub fn run(database_url: &str) {
    match client_command(database_url) {
        Some((program, args)) => {
            let _ = Command::new(program).args(args).status();
        }
        None => doido_core::tracing::error!("no known db client for URL: {database_url}"),
    }
}
