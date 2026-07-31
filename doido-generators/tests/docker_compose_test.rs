use doido_generators::{Generator, ProjectGenerator};

fn find_content<'a>(files: &'a [doido_generators::GeneratedFile], path: &str) -> &'a str {
    &files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("missing {path}"))
        .content
}

#[test]
fn sqlite_compose_has_web_without_database_service() {
    let files = ProjectGenerator
        .generate(&["app", "--database=sqlite"])
        .unwrap();
    let compose = find_content(&files, "app/docker-compose.yml");
    assert!(compose.contains("dockerfile: Dockerfile.dev"));
    assert!(!compose.contains("postgres:"));
    assert!(!compose.contains("mysql:"));
    assert!(!compose.contains("redis:"));
}

#[test]
fn postgres_compose_includes_postgres_service() {
    let files = ProjectGenerator
        .generate(&["blog", "--database=postgres"])
        .unwrap();
    let compose = find_content(&files, "blog/docker-compose.yml");
    assert!(compose.contains("postgres:18-alpine"));
    assert!(compose.contains("@postgres:5432/blog_development"));
    assert!(!compose.contains("redis:"));
}

#[test]
fn mysql_compose_includes_mysql_service() {
    let files = ProjectGenerator
        .generate(&["store", "--database=mysql"])
        .unwrap();
    let compose = find_content(&files, "store/docker-compose.yml");
    assert!(compose.contains("mysql:lts"));
    assert!(compose.contains("@mysql:3306/store_development"));
}

#[test]
fn cache_redis_adds_redis_service_and_feature() {
    let files = ProjectGenerator
        .generate(&["app", "--database=sqlite", "--cache=redis"])
        .unwrap();
    let compose = find_content(&files, "app/docker-compose.yml");
    assert!(compose.contains("redis:8-alpine"));
    assert!(compose.contains("CACHE__ENDPOINT: redis://redis:6379"));

    let cargo = find_content(&files, "app/Cargo.toml");
    assert!(cargo.contains("cache-redis"));

    let dev = find_content(&files, "app/config/development.yml");
    assert!(dev.contains("type: redis"));
    assert!(dev.contains("endpoint: redis://127.0.0.1:6379"));
}

#[test]
fn cache_memcache_adds_memcache_service() {
    let files = ProjectGenerator
        .generate(&["app", "--database=sqlite", "--cache=memcache"])
        .unwrap();
    let compose = find_content(&files, "app/docker-compose.yml");
    assert!(compose.contains("memcached:1.6-alpine"));
    assert!(compose.contains("CACHE__ENDPOINT: memcache://memcache:11211"));
    assert!(!compose.contains("redis:"));
}

#[test]
fn jobs_db_adds_feature_and_config() {
    let files = ProjectGenerator
        .generate(&["app", "--database=postgres", "--jobs=db"])
        .unwrap();
    let cargo = find_content(&files, "app/Cargo.toml");
    assert!(cargo.contains("jobs-db"));

    let dev = find_content(&files, "app/config/development.yml");
    assert!(dev.contains("type: db"));
}

#[test]
fn jobs_redis_adds_redis_service() {
    let files = ProjectGenerator
        .generate(&["app", "--database=sqlite", "--jobs=redis"])
        .unwrap();
    let compose = find_content(&files, "app/docker-compose.yml");
    assert!(compose.contains("redis:8-alpine"));
    assert!(compose.contains("JOBS__REDIS__URL: redis://redis:6379"));
}

#[test]
fn cable_and_jobs_redis_share_one_redis_service() {
    let files = ProjectGenerator
        .generate(&["app", "--database=sqlite", "--cable", "--jobs=redis"])
        .unwrap();
    let compose = find_content(&files, "app/docker-compose.yml");
    assert_eq!(compose.matches("image: redis:").count(), 1);
}

#[test]
fn test_yml_always_uses_memory_backends() {
    let files = ProjectGenerator
        .generate(&[
            "app",
            "--database=postgres",
            "--cache=redis",
            "--jobs=redis",
        ])
        .unwrap();
    let test = find_content(&files, "app/config/test.yml");
    assert!(test.contains("cache:\n  # Tests"));
    assert!(test.contains("type: memory"));
    assert!(test.contains("jobs:\n  type: memory"));
}

#[test]
fn rejects_unknown_cache_backend() {
    let err = ProjectGenerator
        .generate(&["app", "--cache=invalid"])
        .unwrap_err();
    assert!(err.to_string().contains("cache"));
}

#[test]
fn jobs_database_alias_is_accepted() {
    let files = ProjectGenerator
        .generate(&["app", "--database=sqlite", "--jobs=database"])
        .unwrap();
    let dev = find_content(&files, "app/config/development.yml");
    assert!(dev.contains("type: db"));
}
