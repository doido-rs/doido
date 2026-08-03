//! CLI helpers for driving the real `doido-generators` binary.

use assert_cmd::Command;
use std::path::Path;
use std::process::{Command as StdCommand, Output};

pub fn doido(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("doido-generators").expect("doido-generators binary");
    cmd.current_dir(dir);
    cmd
}

pub fn run_app(bin: &Path, app: &Path, args: &[&str]) {
    let status = cargo_app_command(bin, app)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("run {args:?}: {e}"));
    assert!(status.success(), "app command {args:?} failed");
}

pub fn run_app_capture(bin: &Path, app: &Path, args: &[&str]) -> Output {
    cargo_app_command(bin, app)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("run {args:?}: {e}"))
}

fn cargo_app_command(bin: &Path, app: &Path) -> StdCommand {
    let mut cmd = StdCommand::new(bin);
    cmd.current_dir(app);
    cmd
}

pub fn cargo_build(app: &Path) {
    let target = super::workspace::app_cargo_target(app);
    std::fs::create_dir_all(&target).expect("create cargo target dir");
    let status = StdCommand::new(env!("CARGO"))
        .args(["build", "--workspace", "--bin", "blog", "--manifest-path"])
        .arg(app.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .env("RUSTFLAGS", "-D warnings")
        .status()
        .expect("cargo build");
    assert!(status.success(), "generated app failed to build");
}

pub fn app_bin(app_name: &str, app: &Path) -> std::path::PathBuf {
    super::workspace::app_cargo_target(app)
        .join("debug")
        .join(app_name)
}
