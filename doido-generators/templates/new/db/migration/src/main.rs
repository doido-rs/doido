use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    // Entry point for the SeaORM migration CLI. Run with, e.g.:
    //   cargo run -- up
    //   cargo run -- down
    // or via `sea-orm-cli migrate` from the application root.
    cli::run_cli(migration::Migrator).await;
}
