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

#[path = "../app/services/mod.rs"]
mod services;

#[path = "../app/state.rs"]
mod state;

#[path = "../db/migration/mod.rs"]
mod migration;

#[path = "../db/seeds.rs"]
mod seed;

#[path = "../config/routes.rs"]
mod routes;

#[tokio::main]
async fn main() {
    doido::Doido::new()
        .router(routes::router())
        .migrator::<migration::Migrator>()
        .seeder(seed::run)
        .run()
        .await;
}
