//! Convenience entry point for booting an application's HTTP server.

use crate::config;

/// Boots the HTTP server for `router`.
///
/// The listen address is the `server.bind` IP joined with `server.port` from
/// `config/<env>.yml` (the environment comes from [`crate::Environment::get_env`]).
/// When no config file is present the defaults `0.0.0.0:3000` are used.
pub async fn start_server(router: axum::Router) -> std::io::Result<()> {
    let config = config::load();
    let server = config.server();
    let addr = format!("{}:{}", server.bind, server.port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("listening on http://{addr}");
    axum::serve(listener, router).await
}
