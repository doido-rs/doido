//! `new_options` backend parsing + `as_str` for every variant, including the
//! aliases and the unknown-value error paths.

use doido_generators::new_options::{
    parse_cache, parse_database, parse_jobs, CacheBackend, DatabaseBackend, JobsBackend,
};

#[test]
fn database_parse_and_as_str() {
    assert_eq!(parse_database("sqlite").unwrap(), DatabaseBackend::Sqlite);
    assert_eq!(parse_database("postgres").unwrap(), DatabaseBackend::Postgres);
    assert_eq!(
        parse_database("postgresql").unwrap(),
        DatabaseBackend::Postgres
    );
    assert_eq!(parse_database("mysql").unwrap(), DatabaseBackend::Mysql);
    assert!(parse_database("oracle").is_err());

    assert_eq!(DatabaseBackend::Sqlite.as_str(), "sqlite");
    assert_eq!(DatabaseBackend::Postgres.as_str(), "postgres");
    assert_eq!(DatabaseBackend::Mysql.as_str(), "mysql");
}

#[test]
fn cache_parse_and_as_str() {
    assert_eq!(parse_cache("memory").unwrap(), CacheBackend::Memory);
    assert_eq!(parse_cache("redis").unwrap(), CacheBackend::Redis);
    assert_eq!(parse_cache("memcache").unwrap(), CacheBackend::Memcache);
    assert_eq!(parse_cache("memcached").unwrap(), CacheBackend::Memcache);
    assert!(parse_cache("nope").is_err());

    assert_eq!(CacheBackend::Memory.as_str(), "memory");
    assert_eq!(CacheBackend::Redis.as_str(), "redis");
    assert_eq!(CacheBackend::Memcache.as_str(), "memcache");
}

#[test]
fn jobs_parse_and_as_str() {
    assert_eq!(parse_jobs("memory").unwrap(), JobsBackend::Memory);
    assert_eq!(parse_jobs("in_memory").unwrap(), JobsBackend::Memory);
    assert_eq!(parse_jobs("db").unwrap(), JobsBackend::Db);
    assert_eq!(parse_jobs("database").unwrap(), JobsBackend::Db);
    assert_eq!(parse_jobs("sql").unwrap(), JobsBackend::Db);
    assert_eq!(parse_jobs("redis").unwrap(), JobsBackend::Redis);
    assert!(parse_jobs("kafka").is_err());

    assert_eq!(JobsBackend::Memory.as_str(), "memory");
    assert_eq!(JobsBackend::Db.as_str(), "db");
    assert_eq!(JobsBackend::Redis.as_str(), "redis");
}
