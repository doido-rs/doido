#[path = "../app/controllers/mod.rs"]
mod controllers;

#[path = "../config/routes.rs"]
mod routes;

#[tokio::main]
async fn main() {
    // Delegates to the Doido CLI (server, console, db, worker, generate, …),
    // handing it this app's routes so `doido server` can boot the HTTP server.
    doido::run(Some(routes::router())).await;
}
