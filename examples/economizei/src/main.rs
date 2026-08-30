#[path = "../app/controllers/mod.rs"]
mod controllers;

#[path = "../app/models/mod.rs"]
mod models;

#[path = "../app/services/mod.rs"]
mod services;

#[path = "../app/jobs/mod.rs"]
mod jobs;

#[path = "../app/mailers/mod.rs"]
mod mailers;

#[path = "../app/boot.rs"]
mod boot;

#[path = "../db/seeds.rs"]
mod seed;

#[path = "../config/routes.rs"]
mod routes;

#[tokio::main]
async fn main() {
    boot::schedule_startup_jobs_if_server();
    doido::Doido::new()
        .router(routes::router())
        .migrator::<migration::Migrator>()
        .seeder(seed::run)
        .run()
        .await;
}
