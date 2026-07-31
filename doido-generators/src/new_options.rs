//! Typed options for `doido new` — shared between the CLI and the project generator.

use clap::ValueEnum;
use doido_core::{anyhow, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum DatabaseBackend {
    Sqlite,
    Postgres,
    Mysql,
}

impl DatabaseBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum CacheBackend {
    Memory,
    Redis,
    Memcache,
}

impl CacheBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Redis => "redis",
            Self::Memcache => "memcache",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum JobsBackend {
    Memory,
    Db,
    Redis,
}

impl JobsBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Db => "db",
            Self::Redis => "redis",
        }
    }
}

pub fn parse_database(s: &str) -> Result<DatabaseBackend> {
    match s.trim().to_ascii_lowercase().as_str() {
        "sqlite" => Ok(DatabaseBackend::Sqlite),
        "postgres" | "postgresql" => Ok(DatabaseBackend::Postgres),
        "mysql" => Ok(DatabaseBackend::Mysql),
        other => Err(anyhow::anyhow!(
            "Unknown database: {other}. Use sqlite, postgres, or mysql."
        )),
    }
}

pub fn parse_cache(s: &str) -> Result<CacheBackend> {
    match s.trim().to_ascii_lowercase().as_str() {
        "memory" => Ok(CacheBackend::Memory),
        "redis" => Ok(CacheBackend::Redis),
        "memcache" | "memcached" => Ok(CacheBackend::Memcache),
        other => Err(anyhow::anyhow!(
            "Unknown cache backend: {other}. Use memory, redis, or memcache."
        )),
    }
}

pub fn parse_jobs(s: &str) -> Result<JobsBackend> {
    match s.trim().to_ascii_lowercase().as_str() {
        "memory" | "inmemory" | "in_memory" => Ok(JobsBackend::Memory),
        "db" | "database" | "sql" => Ok(JobsBackend::Db),
        "redis" => Ok(JobsBackend::Redis),
        other => Err(anyhow::anyhow!(
            "Unknown jobs backend: {other}. Use memory, db, or redis."
        )),
    }
}
