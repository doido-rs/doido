//! Per-command idempotency flags for migration helpers.

use crate::sea_orm::{DatabaseBackend, DbErr};

/// Options for migration DSL commands (Rails-style `if_not_exists` / `if_exists`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CmdOpts {
    /// For create/add: tolerate or skip when the object already exists.
    pub if_not_exists: bool,
    /// For drop/remove: tolerate or skip when the object is absent.
    pub if_exists: bool,
}

impl CmdOpts {
    pub const fn new() -> Self {
        Self {
            if_not_exists: false,
            if_exists: false,
        }
    }

    /// Only set `IF NOT EXISTS` (or equivalent) on create/add operations.
    pub const fn if_not_exists() -> Self {
        Self {
            if_not_exists: true,
            if_exists: false,
        }
    }

    /// Only set `IF EXISTS` (or equivalent) on drop/remove operations.
    pub const fn if_exists() -> Self {
        Self {
            if_not_exists: false,
            if_exists: true,
        }
    }

    /// Idempotent creates/adds and tolerant drops/removes (historical defaults).
    pub const fn idempotent() -> Self {
        Self {
            if_not_exists: true,
            if_exists: true,
        }
    }
}

impl Default for CmdOpts {
    fn default() -> Self {
        Self::idempotent()
    }
}

pub(crate) fn is_duplicate_index_err(backend: DatabaseBackend, err: &DbErr) -> bool {
    if !matches!(backend, DatabaseBackend::MySql) {
        return false;
    }
    let msg = err.to_string().to_lowercase();
    msg.contains("duplicate key name") || msg.contains("1061")
}

pub(crate) fn is_duplicate_column_err(backend: DatabaseBackend, err: &DbErr) -> bool {
    let msg = err.to_string().to_lowercase();
    match backend {
        DatabaseBackend::MySql => msg.contains("duplicate column name") || msg.contains("1060"),
        DatabaseBackend::Postgres => msg.contains("already exists"),
        DatabaseBackend::Sqlite => msg.contains("duplicate column name"),
        _ => false,
    }
}

pub(crate) fn is_missing_column_err(backend: DatabaseBackend, err: &DbErr) -> bool {
    let msg = err.to_string().to_lowercase();
    match backend {
        DatabaseBackend::MySql => {
            msg.contains("check that column/key exists") || msg.contains("1091")
        }
        DatabaseBackend::Postgres => msg.contains("does not exist"),
        DatabaseBackend::Sqlite => msg.contains("no such column"),
        _ => false,
    }
}

pub(crate) fn is_duplicate_foreign_key_err(backend: DatabaseBackend, err: &DbErr) -> bool {
    let msg = err.to_string().to_lowercase();
    match backend {
        DatabaseBackend::MySql => msg.contains("duplicate foreign key") || msg.contains("1826"),
        DatabaseBackend::Postgres => msg.contains("already exists"),
        DatabaseBackend::Sqlite => msg.contains("already exists"),
        _ => false,
    }
}

pub(crate) fn is_missing_index_err(backend: DatabaseBackend, err: &DbErr) -> bool {
    let msg = err.to_string().to_lowercase();
    match backend {
        DatabaseBackend::MySql => {
            msg.contains("check that column/key exists")
                || msg.contains("1091")
                || msg.contains("can't drop")
        }
        DatabaseBackend::Postgres => msg.contains("does not exist"),
        DatabaseBackend::Sqlite => msg.contains("no such index"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::DbErr;

    #[test]
    fn default_opts_are_idempotent() {
        assert_eq!(CmdOpts::default(), CmdOpts::idempotent());
    }

    #[test]
    fn mysql_duplicate_index_detection() {
        let err = DbErr::Custom("Error 1061 Duplicate key name 'idx_x'".into());
        assert!(is_duplicate_index_err(DatabaseBackend::MySql, &err));
    }
}
