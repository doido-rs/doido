//! `ProjectGenerator` with optional features enabled — exercises the auth,
//! Redis (cache + jobs), cable, and memcache branches of `new.rs` that the
//! default-flags tests don't reach.

use doido_generators::generators::new::ProjectGenerator;
use doido_generators::Generator;

fn file<'a>(files: &'a [doido_generators::GeneratedFile], suffix: &str) -> &'a str {
    files
        .iter()
        .find(|f| f.path.ends_with(suffix))
        .unwrap_or_else(|| panic!("no generated file ending in {suffix}"))
        .content
        .as_str()
}

#[test]
fn kitchen_sink_wires_auth_redis_and_cable() {
    let files = ProjectGenerator
        .generate(&[
            "blog",
            "--database=postgres",
            "--cache=redis",
            "--jobs=redis",
            "--cable",
            "--auth",
        ])
        .unwrap();

    let cargo = file(&files, "blog/Cargo.toml");
    assert!(cargo.contains("cache-redis"), "cache-redis feature");
    assert!(cargo.contains("jobs-redis"), "jobs-redis feature");
    assert!(cargo.contains("doido-auth"), "auth dependency");
    assert!(cargo.contains("doido-cable"), "cable dependency");

    let dev = file(&files, "blog/config/development.yml");
    assert!(dev.contains("type: redis"), "redis cache/jobs config");

    let compose = file(&files, "blog/docker-compose.yml");
    assert!(compose.contains("redis:8-alpine"), "redis compose service");
}

#[test]
fn memcache_cache_wires_memcache_service_and_feature() {
    let files = ProjectGenerator
        .generate(&["shop", "--database=mysql", "--cache=memcache"])
        .unwrap();

    let cargo = file(&files, "shop/Cargo.toml");
    assert!(cargo.contains("cache-memcache"), "cache-memcache feature");

    let dev = file(&files, "shop/config/development.yml");
    assert!(dev.contains("memcache"), "memcache cache config");

    let compose = file(&files, "shop/docker-compose.yml");
    assert!(compose.contains("memcache"), "memcache compose service");
}

#[test]
fn jobs_db_backend_shares_database_feature() {
    let files = ProjectGenerator
        .generate(&["site", "--database=postgres", "--jobs=db"])
        .unwrap();
    let cargo = file(&files, "site/Cargo.toml");
    assert!(cargo.contains("jobs-db"), "jobs-db feature");
    let dev = file(&files, "site/config/development.yml");
    assert!(dev.contains("jobs:"), "jobs config section");
}
