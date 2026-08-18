//! Integration tests for `doido db` subcommands (SQLite, in-memory).

use assert_cmd::Command;
use doido_model::commands::db::seed_command;
use doido_generators::generators::new::ProjectGenerator;
use doido_generators::Generator;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("doido").unwrap()
}

fn write_generated_app(dir: &Path, name: &str) {
    let files = ProjectGenerator
        .generate(&[name, "--database=sqlite"])
        .expect("generate app");
    for file in files {
        let rel = file
            .path
            .strip_prefix(&format!("{name}/"))
            .unwrap_or(file.path.as_str());
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, file.content).unwrap();
    }
}

fn sqlite_app() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_generated_app(dir.path(), "app");
    fs::create_dir_all(dir.path().join("db")).unwrap();
    fs::write(
        dir.path().join("db/schema.sql"),
        "CREATE TABLE items (id INTEGER PRIMARY KEY);",
    )
    .unwrap();
    dir
}

#[test]
fn seed_command_runs_the_seed_crate() {
    let (program, args) = seed_command();
    assert_eq!(program, "cargo");
    assert_eq!(
        args,
        vec!["run", "--quiet", "--manifest-path", "db/seed/Cargo.toml"]
    );
}

#[test]
fn db_prepare_loads_schema_when_empty() {
    let dir = sqlite_app();
    cmd()
        .current_dir(dir.path())
        .env("DATABASE_URL", "sqlite::memory:")
        .args(["db", "prepare"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prepared database"));
}

#[test]
fn db_seed_runs_seed_crate() {
    let dir = sqlite_app();
    cmd()
        .current_dir(dir.path())
        .env("DATABASE_URL", "sqlite::memory:")
        .args(["db", "seed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("seeded database via db/seed"));
}

#[test]
fn db_reset_reloads_schema() {
    let dir = sqlite_app();
    cmd()
        .current_dir(dir.path())
        .env("DATABASE_URL", "sqlite::memory:")
        .args(["db", "reset"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reset database"));
}

#[test]
fn db_schema_dump_and_load() {
    let dir = sqlite_app();
    cmd()
        .current_dir(dir.path())
        .env("DATABASE_URL", "sqlite::memory:")
        .args(["db", "prepare"])
        .assert()
        .success();
    cmd()
        .current_dir(dir.path())
        .env("DATABASE_URL", "sqlite::memory:")
        .args(["db", "schema", "dump"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote schema"));
    assert!(dir.path().join("db/schema.sql").exists());

    cmd()
        .current_dir(dir.path())
        .env("DATABASE_URL", "sqlite::memory:")
        .args(["db", "schema", "load"])
        .assert()
        .success()
        .stdout(predicate::str::contains("loaded schema"));
}

#[test]
fn db_create_sqlite_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("db")).unwrap();
    let db_path = dir.path().join("db/test.db");
    cmd()
        .current_dir(dir.path())
        .env("DATABASE_URL", format!("sqlite:file:{}", db_path.display()))
        .args(["db", "create"])
        .assert()
        .success()
        .stdout(predicate::str::contains("database"));
}

#[test]
fn db_prepare_seeds_database_url_from_config_yaml() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/development.yml"),
        "database:\n  url: \"sqlite::memory:\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("db")).unwrap();
    fs::write(
        dir.path().join("db/schema.sql"),
        "CREATE TABLE items (id INTEGER PRIMARY KEY);",
    )
    .unwrap();
    cmd()
        .current_dir(dir.path())
        .env_remove("DATABASE_URL")
        .env("DOIDO_ENV", "development")
        .args(["db", "prepare"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prepared database"));
}

#[test]
fn db_missing_schema_file_logs_error() {
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .current_dir(dir.path())
        .env("DATABASE_URL", "sqlite::memory:")
        .args(["db", "reset"])
        .assert()
        .success()
        .stdout(predicate::str::contains("could not read"));
}
