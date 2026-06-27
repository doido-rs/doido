#[path = "../app/controllers/mod.rs"]
mod controllers;

#[path = "../config/routes.rs"]
mod routes;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Binds the IP from `SERVER_BIND` (default 0.0.0.0) joined with the port
    // from `SERVER_PORT` (default 3000). Override either via the environment.
    doido::controller::start_server(routes::router()).await
}
