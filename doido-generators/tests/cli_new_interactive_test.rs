use assert_cmd::Command as AssertCommand;
use std::io::Write;
use std::process::{Command, Stdio};

fn bin_path() -> std::path::PathBuf {
    AssertCommand::cargo_bin("doido")
        .unwrap()
        .get_program()
        .into()
}

#[test]
fn test_new_non_interactive_uses_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new(bin_path())
        .current_dir(dir.path())
        .args(["new", "defaults-app", "--non-interactive"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(dir.path().join("defaults-app/Cargo.toml").exists());
}

#[test]
fn test_new_interactive_accepts_default_answers() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(bin_path())
        .current_dir(dir.path())
        .args(["new", "prompt-app"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let stdin = child.stdin.as_mut().unwrap();
        // database, cache, jobs (empty lines = defaults), cable (empty = no)
        stdin.write_all(b"\n\n\n\n").unwrap();
    }

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("prompt-app/Cargo.toml").exists());
}
