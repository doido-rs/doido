use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("doido-generators").unwrap()
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
fn test_server_command_without_routes_does_not_start() {
    // The standalone binary passes `None` for routes, so the server must not
    // start — it just reports there is nothing to serve.
    cmd()
        .arg("server")
        .assert()
        .success()
        .stdout(predicate::str::contains("server not started"));
}

#[test]
fn test_db_help_lists_subcommands() {
    cmd()
        .args(["db", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("migrate"))
        .stdout(predicate::str::contains("generate"));
}

#[test]
fn test_db_migrate_help_lists_subcommands() {
    cmd()
        .args(["db", "migrate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Apply pending migrations"))
        .stdout(predicate::str::contains("Rollback applied migrations"));
}

#[test]
fn test_db_generate_entity_help_available() {
    cmd()
        .args(["db", "generate", "entity", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("output"));
}

#[test]
fn test_jobs_failed_command() {
    cmd()
        .args(["jobs", "failed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("failed jobs"));
}

#[test]
fn test_generate_controller() {
    // Run in a tempdir: `generate` writes files relative to the cwd, so running
    // it in the package root would pollute the crate's own source tree.
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .current_dir(dir.path())
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
fn test_generate_empty_lists_generators() {
    cmd()
        .arg("generate")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available generators"))
        .stdout(predicate::str::contains("controller"))
        .stdout(predicate::str::contains("scaffold"));
}

#[test]
fn test_generate_help_lists_generators() {
    cmd()
        .args(["generate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available generators"))
        .stdout(predicate::str::contains("templates"));
}

#[test]
fn test_worker_command() {
    // `--once` drains ready jobs and exits, so the command terminates (the
    // default mode runs until Ctrl-C).
    cmd()
        .args(["worker", "--once"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("worker"));
}

#[test]
fn test_credentials_edit_command() {
    // Run non-interactively: `true` stands in for $EDITOR (leaves the file as-is
    // and exits 0), an explicit master key avoids touching the repo, and a temp
    // cwd contains the encrypted file it writes. The timeout guards against a
    // hang if the editor stub ever blocks.
    let dir = std::env::temp_dir().join(format!(
        "doido-cli-cred-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    cmd()
        .args(["credentials", "edit"])
        .current_dir(&dir)
        .env("EDITOR", "true")
        .env("DOIDO_MASTER_KEY", "cli-test-master-key")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("credentials"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_credentials_show_command() {
    // With a master key but no credentials file yet, `show` prints the seed
    // template (which mentions "credentials") and exits 0.
    let dir = std::env::temp_dir().join(format!(
        "doido-cli-cred-show-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    cmd()
        .args(["credentials", "show"])
        .current_dir(&dir)
        .env("DOIDO_MASTER_KEY", "cli-test-master-key")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("credentials"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_console_command() {
    cmd()
        .arg("console")
        .assert()
        .success()
        .stdout(predicate::str::contains("console"));
}
