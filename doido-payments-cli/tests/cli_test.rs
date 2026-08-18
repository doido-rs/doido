//! Integration tests for the `doido-payments` CLI binary (full Doido CLI + payments).

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn doido_payments_generate_lists_payment_generators() {
    Command::cargo_bin("doido-payments")
        .unwrap()
        .args(["generate"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Payments (doido-payments):"))
        .stdout(predicate::str::contains("payments:install"))
        .stdout(predicate::str::contains("payments:scaffold"));
}

#[test]
fn doido_payments_has_doido_server_command() {
    Command::cargo_bin("doido-payments")
        .unwrap()
        .arg("server")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Start the web server"));
}

#[test]
fn doido_payments_has_doido_new_command() {
    Command::cargo_bin("doido-payments")
        .unwrap()
        .arg("new")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Create a new Doido application"));
}
