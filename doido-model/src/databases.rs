//! Multiple databases / read-write splitting (Rails `connects_to database:
//! { writing:, reading: }`).
//!
//! [`Databases`] holds a primary (writing) connection and an optional replica
//! (reading) connection; [`Databases::connection`] routes by [`Role`], falling
//! back to the writer when no replica is configured.

use crate::sea_orm::DatabaseConnection;

/// Which connection a query should use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Writing,
    Reading,
}

/// A primary connection plus an optional read replica.
pub struct Databases {
    writing: DatabaseConnection,
    reading: Option<DatabaseConnection>,
}

impl Databases {
    /// A single-database setup (all queries hit `writing`).
    pub fn new(writing: DatabaseConnection) -> Self {
        Self {
            writing,
            reading: None,
        }
    }

    /// Add a read replica; reads then route to it.
    pub fn with_reading(mut self, reading: DatabaseConnection) -> Self {
        self.reading = Some(reading);
        self
    }

    /// Whether a read replica is configured.
    pub fn has_replica(&self) -> bool {
        self.reading.is_some()
    }

    /// The connection for `role` — reads fall back to the writer without a replica.
    pub fn connection(&self, role: Role) -> &DatabaseConnection {
        match role {
            Role::Writing => &self.writing,
            Role::Reading => self.reading.as_ref().unwrap_or(&self.writing),
        }
    }

    /// The writing connection.
    pub fn writing(&self) -> &DatabaseConnection {
        &self.writing
    }

    /// The reading connection (falls back to the writer).
    pub fn reading(&self) -> &DatabaseConnection {
        self.connection(Role::Reading)
    }
}
