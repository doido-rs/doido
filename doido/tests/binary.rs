use assert_cmd::Command;

#[test]
fn binary_prints_help() {
    Command::cargo_bin("doido")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("doido"));
}

#[test]
fn binary_server_subcommand_exists() {
    Command::cargo_bin("doido")
        .unwrap()
        .arg("server")
        .assert()
        .success();
}
