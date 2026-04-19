use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("doido").unwrap()
}

#[test]
fn test_help_exits_zero() {
    cmd().arg("--help").assert().success();
}

#[test]
fn test_version_output() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_server_command() {
    cmd()
        .arg("server")
        .assert()
        .success()
        .stdout(predicate::str::contains("Starting server"));
}

#[test]
fn test_db_migrate_command() {
    cmd()
        .args(["db", "migrate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("migrations"));
}

#[test]
fn test_db_rollback_command() {
    cmd()
        .args(["db", "rollback"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rolling back"));
}

#[test]
fn test_jobs_failed_command() {
    cmd()
        .args(["jobs", "failed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Failed jobs"));
}

#[test]
fn test_generate_controller() {
    cmd()
        .args(["generate", "controller", "Posts"])
        .assert()
        .success()
        .stdout(predicate::str::contains("posts_controller.rs"));
}

#[test]
fn test_generate_unknown_generator() {
    cmd()
        .args(["generate", "nonexistent", "Foo"])
        .assert()
        .failure();
}

#[test]
fn test_worker_command() {
    cmd()
        .arg("worker")
        .assert()
        .success()
        .stdout(predicate::str::contains("worker"));
}

#[test]
fn test_credentials_edit_command() {
    cmd()
        .args(["credentials", "edit"])
        .assert()
        .success()
        .stdout(predicate::str::contains("credentials"));
}

#[test]
fn test_console_command() {
    cmd()
        .arg("console")
        .assert()
        .success()
        .stdout(predicate::str::contains("console"));
}
