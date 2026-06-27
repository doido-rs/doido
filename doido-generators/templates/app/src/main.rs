#[path = "../app/controllers/mod.rs"]
mod controllers;

#[path = "../config/routes.rs"]
mod routes;

/// IP the server binds to when `SERVER_BIND` is unset.
const DEFAULT_BIND: &str = "0.0.0.0";
/// Port the server listens on when `SERVER_PORT` is unset.
const DEFAULT_PORT: u16 = 3000;

/// Boots the HTTP server on `bind` (an IP) joined with `port`, e.g. `0.0.0.0:3000`.
async fn start_server(bind: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{bind}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("listening on http://{addr}");
    doido::controller::axum::serve(listener, routes::router()).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind = std::env::var("SERVER_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());
    let port = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    start_server(&bind, port).await
}
