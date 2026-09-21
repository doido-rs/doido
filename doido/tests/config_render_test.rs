//! `doido config render` for deploy-time YAML materialization.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn config_render_expands_get_env_to_stdout() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/production.yml"),
        "database:\n  url: {{ get_env(name=\"DATABASE_URL\", default=\"sqlite://x\") }}\n",
    )
    .unwrap();

    std::env::set_var("DATABASE_URL", "postgres://rendered/db");
    Command::new(env!("CARGO_BIN_EXE_doido"))
        .current_dir(dir.path())
        .args(["config", "render", "--env", "production"])
        .assert()
        .success()
        .stdout(predicate::str::contains("postgres://rendered/db"));
    std::env::remove_var("DATABASE_URL");
}
