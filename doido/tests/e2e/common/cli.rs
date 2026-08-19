//! CLI helpers for driving the real `doido` binary.

use assert_cmd::Command;
use std::path::Path;
use std::process::{Command as StdCommand, Output};
use std::sync::OnceLock;

use super::workspace::{self, BaseProfile};

pub fn doido(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("doido").expect("doido binary");
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
    let target = workspace::shared_cargo_target();
    std::fs::create_dir_all(&target).ok();
    let mut cmd = StdCommand::new(bin);
    cmd.current_dir(app)
        .env("CARGO_TARGET_DIR", target)
        .env("RUSTFLAGS", "-D warnings");
    cmd
}

fn cargo_env(app: &Path) -> StdCommand {
    let target = workspace::shared_cargo_target();
    std::fs::create_dir_all(&target).expect("create shared cargo target dir");
    let mut cmd = StdCommand::new(env!("CARGO"));
    cmd.current_dir(app)
        .env("CARGO_TARGET_DIR", &target)
        .env("RUSTFLAGS", "-D warnings");
    cmd
}

pub fn cargo_build(app: &Path) {
    let status = cargo_env(app)
        .args(["build", "--workspace", "--bin", "blog", "--manifest-path"])
        .arg(app.join("Cargo.toml"))
        .status()
        .expect("cargo build");
    assert!(status.success(), "generated app failed to build");
}

/// Builds each baseline profile once so later scenario forks reuse linked deps.
pub fn warm_base_profile(profile: BaseProfile) {
    static DEFAULT: OnceLock<()> = OnceLock::new();
    static API_ONLY: OnceLock<()> = OnceLock::new();
    static CABLE: OnceLock<()> = OnceLock::new();
    static JOBS_DB: OnceLock<()> = OnceLock::new();
    static AUTH_API: OnceLock<()> = OnceLock::new();
    static AUTH_HTML: OnceLock<()> = OnceLock::new();

    let warm = || {
        let app = workspace::profile_base_app(profile);
        if app.join("Cargo.toml").is_file() {
            cargo_build(&app);
        }
    };

    match profile {
        BaseProfile::Default => {
            DEFAULT.get_or_init(warm);
        }
        BaseProfile::ApiOnly => {
            API_ONLY.get_or_init(warm);
        }
        BaseProfile::WithCable => {
            CABLE.get_or_init(warm);
        }
        BaseProfile::WithJobsDb => {
            JOBS_DB.get_or_init(warm);
        }
        BaseProfile::WithAuthApi => {
            AUTH_API.get_or_init(warm);
        }
        BaseProfile::WithAuthHtml => {
            AUTH_HTML.get_or_init(warm);
        }
    }
}

pub fn cargo_test(app: &Path, args: &[&str]) -> std::process::ExitStatus {
    cargo_env(app)
        .args(["test"])
        .args(args)
        .status()
        .expect("cargo test")
}

pub fn app_bin(app_name: &str, _app: &Path) -> std::path::PathBuf {
    workspace::shared_cargo_target()
        .join("debug")
        .join(app_name)
}
