//! Shared paths and app skeleton caching for e2e runs.

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Workspace root (`doido/` checkout).
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Per-app `CARGO_TARGET_DIR` so generated apps with the same package name (`blog`)
/// do not overwrite each other's binaries in a shared target directory.
pub fn app_cargo_target(app: &Path) -> PathBuf {
    app.join("target")
}

/// Root directory for forked scenario apps (kept when `E2E_KEEP=1`).
pub fn e2e_apps_root() -> PathBuf {
    workspace_root().join("target/e2e/apps")
}

/// Serializes e2e tests that mutate shared workspace state.
pub fn e2e_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Baseline `doido new` profile cached on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseProfile {
    Default,
    WithCable,
    WithAuth,
}

impl BaseProfile {
    fn cache_dir(self) -> PathBuf {
        let name = match self {
            Self::Default => "default",
            Self::WithCable => "cable",
            Self::WithAuth => "auth",
        };
        e2e_apps_root().join("_base").join(name)
    }

    fn new_args(self) -> Vec<&'static str> {
        let mut args = vec!["new", "blog", "--non-interactive", "--database=sqlite"];
        if self == Self::WithCable {
            args.push("--cable");
        }
        if self == Self::WithAuth {
            args.push("--auth");
        }
        args
    }
}

/// Returns a fresh copy of the cached baseline app for `scenario`.
pub fn fork_scenario(scenario: &str, profile: BaseProfile) -> PathBuf {
    fs::create_dir_all(e2e_apps_root()).expect("create e2e apps root");
    let base = ensure_base_app(profile);
    let dest = e2e_apps_root().join(scenario);
    if dest.exists() {
        if std::env::var_os("E2E_KEEP").is_some() {
            return dest;
        }
        fs::remove_dir_all(&dest).ok();
    }
    copy_dir_excluding(&base, &dest, &["target"]).expect("fork scenario app");
    clear_sqlite_databases(&dest.join("blog"));
    dest
}

/// Remove per-env SQLite files so each scenario starts with a clean database.
fn clear_sqlite_databases(app: &Path) {
    for name in ["development.db", "test.db", "production.db"] {
        let path = app.join("db").join(name);
        if path.is_file() {
            fs::remove_file(path).ok();
        }
    }
}

fn auth_base_is_valid(dir: &Path) -> bool {
    let app = dir.join("blog");
    let routes = app.join("config/routes.rs");
    let sessions = app.join("app/controllers/auth/sessions_controller.rs");
    let user_model = app.join("app/models/user.rs");
    app.join("Cargo.toml").is_file()
        && fs::read_to_string(&routes)
            .map(|content| content.contains("SessionsController::create"))
            .unwrap_or(false)
        && fs::read_to_string(&sessions)
            .map(|content| {
                content.contains("sign_in(ctx, &user)")
                    && content.contains("doido::controller::Context")
            })
            .unwrap_or(false)
        && fs::read_to_string(&user_model)
            .map(|content| content.contains(".map_err(Into::into)"))
            .unwrap_or(false)
}

fn recreate_auth_base(dir: &Path) {
    if dir.exists() {
        fs::remove_dir_all(dir).ok();
    }
    fs::create_dir_all(dir).expect("create auth base dir");
    doido(dir)
        .args(BaseProfile::WithAuth.new_args())
        .assert()
        .success();
}

fn ensure_base_app(profile: BaseProfile) -> PathBuf {
    static DEFAULT: OnceLock<PathBuf> = OnceLock::new();
    static CABLE: OnceLock<PathBuf> = OnceLock::new();

    if profile == BaseProfile::WithAuth {
        let dir = profile.cache_dir();
        if !auth_base_is_valid(&dir) {
            recreate_auth_base(&dir);
        }
        return dir;
    }

    let init = || {
        let dir = profile.cache_dir();
        let app = dir.join("blog");
        if !app.join("Cargo.toml").is_file() {
            fs::create_dir_all(&dir).expect("create base dir");
            doido(&dir).args(&profile.new_args()).assert().success();
        }
        dir
    };

    match profile {
        BaseProfile::Default => DEFAULT.get_or_init(init).clone(),
        BaseProfile::WithCable => CABLE.get_or_init(init).clone(),
        BaseProfile::WithAuth => unreachable!("handled above"),
    }
}

fn doido(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("doido-generators").expect("doido-generators binary");
    cmd.current_dir(dir);
    cmd
}

fn copy_dir_excluding(src: &Path, dst: &Path, exclude: &[&str]) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if exclude.iter().any(|e| name_str == *e) {
            continue;
        }
        let from = entry.path();
        let to = dst.join(name);
        if from.is_dir() {
            copy_dir_excluding(&from, &to, exclude)?;
        } else {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
