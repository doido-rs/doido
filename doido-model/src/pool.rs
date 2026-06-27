//! Database connections backed by the model [`config`](crate::config).

use crate::config;
use sea_orm::{Database, DatabaseConnection, DbErr};

/// Connects to the database named by the current environment's
/// `config/<env>.yml` `database.url`.
pub async fn connect() -> Result<DatabaseConnection, DbErr> {
    let config = config::load();
    Database::connect(config.database().url.as_str()).await
}

/// Connects using an explicit database URL, bypassing the config file.
pub async fn connect_with_url(url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(url).await
}
