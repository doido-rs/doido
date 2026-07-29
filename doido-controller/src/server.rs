//! Convenience entry point for booting an application's HTTP server.

use crate::config::{self, ServerConfig};
use crate::stack::MiddlewareStack;

/// Resolve the listen address, letting explicit `bind`/`port` overrides (e.g.
/// `doido server --port 4000`) win over `config/<env>.yml`.
fn resolve_addr(server: &ServerConfig, bind: Option<&str>, port: Option<u16>) -> String {
    let bind = bind.unwrap_or(&server.bind);
    let port = port.unwrap_or(server.port);
    format!("{bind}:{port}")
}

/// Boots the HTTP server for `router` using `config/<env>.yml` (the environment
/// comes from [`doido_core::Environment::get_env`]). Missing config falls back
/// to `0.0.0.0:3000`.
pub async fn start_server(router: axum::Router) -> std::io::Result<()> {
    start_server_with(router, None, None).await
}

/// Like [`start_server`], but with optional `bind`/`port` overrides that take
/// precedence over the config file (used by `doido server --port`/`--bind`).
pub async fn start_server_with(
    router: axum::Router,
    bind: Option<String>,
    port: Option<u16>,
) -> std::io::Result<()> {
    let config = config::load();
    let addr = resolve_addr(config.server(), bind.as_deref(), port);

    // Apply the always-on middleware (request/response logging + panic
    // recovery) so every request is traced through the global subscriber.
    let router = MiddlewareStack::default().apply(router);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    tracing::info!("routes:\n{}", crate::route_table::format_routes());
    axum::serve(listener, router).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_addr_prefers_overrides_then_config() {
        let server = ServerConfig {
            bind: "0.0.0.0".into(),
            port: 3000,
        };
        assert_eq!(resolve_addr(&server, None, None), "0.0.0.0:3000");
        assert_eq!(resolve_addr(&server, None, Some(4000)), "0.0.0.0:4000");
        assert_eq!(
            resolve_addr(&server, Some("127.0.0.1"), None),
            "127.0.0.1:3000"
        );
        assert_eq!(
            resolve_addr(&server, Some("127.0.0.1"), Some(8080)),
            "127.0.0.1:8080"
        );
    }
}
