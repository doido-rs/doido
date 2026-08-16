use doido::model::sea_orm_migration::cli::run_cli;

#[tokio::main]
async fn main() {
    // Entry point for the SeaORM migration CLI. Run with, e.g.:
    //   cargo run -- up
    //   cargo run -- down
    // or via `doido db migrate` from the application root.
    run_cli(migration::Migrator).await;
}
