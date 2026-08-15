#[path = "../app/controllers/mod.rs"]
mod controllers;

#[path = "../app/models/mod.rs"]
mod models;

#[path = "../app/jobs/mod.rs"]
mod jobs;

#[path = "../app/mailers/mod.rs"]
mod mailers;

#[path = "../app/helpers/mod.rs"]
mod helpers;

#[path = "../app/channels/mod.rs"]
mod channels;

#[path = "../config/routes.rs"]
mod routes;

#[tokio::main]
async fn main() {
    // Delegates to the Doido CLI (server, console, db, worker, generate, …),
    // handing it this app's routes so `doido server` can boot the HTTP server.
    // The `jobs`/`mailers` modules above are compiled as part of this crate, so
    // generated jobs/mailers are type-checked even before they are wired up.
    doido::run(Some(routes::router())).await;
}
