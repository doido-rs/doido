//! Database seeds (Rails `db:seed` / `db/seeds`). A seed routine is an async
//! closure run against the connection; [`run_seeds`] executes it (the CLI's
//! `doido db seed` calls the app's registered seeder).

use crate::sea_orm::DatabaseConnection;
use doido_core::Result;

/// Run a seeding routine against `conn`.
pub async fn run_seeds(
    conn: &DatabaseConnection,
    seeder: impl AsyncFnOnce(&DatabaseConnection) -> Result<()>,
) -> Result<()> {
    seeder(conn).await
}
