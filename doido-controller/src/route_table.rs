//! Process-global table of the application's routes.
//!
//! The `routes!` macro registers every `(method, path)` it expands via
//! [`register_routes`] as the router is built. Because the generated app builds
//! its router (`routes::router()`) before handing it to the CLI, the table is
//! populated by the time `doido server` or `doido routes` runs — letting both
//! print the route list without introspecting axum's opaque `Router`.

use std::sync::Mutex;

/// One registered route (or method group) of the application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteEntry {
    /// HTTP method(s), e.g. `GET` or `PUT|PATCH`.
    pub method: String,
    /// URL path pattern, e.g. `/posts/{id}`.
    pub path: String,
}

static ROUTES: Mutex<Vec<RouteEntry>> = Mutex::new(Vec::new());

/// Records the application's routes, replacing any previously registered set.
/// Called by `routes!`-generated code.
pub fn register_routes(entries: Vec<RouteEntry>) {
    if let Ok(mut guard) = ROUTES.lock() {
        *guard = entries;
    }
}

/// Returns a snapshot of the registered routes, in declaration order.
pub fn all_routes() -> Vec<RouteEntry> {
    ROUTES.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Renders the route table as aligned `METHOD  PATH` lines.
pub fn format_routes() -> String {
    let routes = all_routes();
    if routes.is_empty() {
        return "No routes defined.".to_string();
    }
    let width = routes.iter().map(|r| r.method.len()).max().unwrap_or(0);
    let mut out = String::new();
    out.push_str(&format!(
        "{:<width$}  {}\n",
        "METHOD",
        "PATH",
        width = width
    ));
    for r in &routes {
        out.push_str(&format!(
            "{:<width$}  {}\n",
            r.method,
            r.path,
            width = width
        ));
    }
    out
}

/// Prints the route table to stdout. This is the `doido routes` command's
/// primary data output (like `ls` listing files), so it is written directly
/// rather than through the logger. Server startup logs the table via tracing
/// instead — see [`format_routes`].
pub fn print_routes() {
    print!("{}", format_routes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_format_round_trip() {
        register_routes(vec![
            RouteEntry {
                method: "GET".to_string(),
                path: "/".to_string(),
            },
            RouteEntry {
                method: "PUT|PATCH".to_string(),
                path: "/posts/{id}".to_string(),
            },
        ]);

        let entries = all_routes();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "/");

        let table = format_routes();
        assert!(table.contains("METHOD"));
        assert!(table.contains("GET        /")); // padded to the wider "PUT|PATCH"
        assert!(table.contains("PUT|PATCH  /posts/{id}"));
    }
}
