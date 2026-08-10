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

/// Shared `CARGO_TARGET_DIR` for every generated e2e app so framework crates link once.
/// Scenarios run serially (`--test-threads=1`), so reusing the `blog` binary name is safe.
pub fn shared_cargo_target() -> PathBuf {
    e2e_apps_root().join("cargo-target")
}

/// Path to the cached baseline app root for `profile` (before scenario fork).
pub fn profile_base_app(profile: BaseProfile) -> PathBuf {
    profile.cache_dir().join("blog")
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
    /// `doido new --api`: project marked `api_only` (no auth). Used to prove the
    /// route macros drop `new`/`edit` from a plain `resources!` at compile time.
    ApiOnly,
    WithCable,
    WithJobsDb,
    WithAuthApi,
    WithAuthHtml,
}

impl BaseProfile {
    fn cache_dir(self) -> PathBuf {
        let name = match self {
            Self::Default => "sqlite-memory",
            Self::ApiOnly => "sqlite-api",
            Self::WithCable => "sqlite-cable",
            Self::WithJobsDb => "sqlite-jobs-db",
            Self::WithAuthApi => "auth-api",
            Self::WithAuthHtml => "auth-html",
        };
        e2e_apps_root().join("_base").join(name)
    }

    fn new_args(self) -> Vec<&'static str> {
        let mut args = vec!["new", "blog", "--non-interactive", "--database=sqlite"];
        match self {
            Self::Default => {}
            Self::ApiOnly => args.push("--api"),
            Self::WithCable => args.push("--cable"),
            Self::WithJobsDb => args.push("--jobs=db"),
            Self::WithAuthApi => {
                args.push("--auth");
                args.push("--api");
            }
            Self::WithAuthHtml => args.push("--auth"),
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

fn base_app_is_valid(dir: &Path, profile: BaseProfile) -> bool {
    let app = dir.join("blog");
    let main_rs = app.join("src/main.rs");
    let hello = app.join("app/controllers/hello_controller.rs");
    let helpers_mod = app.join("app/helpers/mod.rs");

    if !app.join("Cargo.toml").is_file()
        || !helpers_mod.is_file()
        || !app.join("db/seed/Cargo.toml").is_file()
    {
        return false;
    }

    let seed_cargo = fs::read_to_string(app.join("db/seed/Cargo.toml")).unwrap_or_default();
    if !seed_cargo.contains("serde =") {
        return false;
    }

    let main_content = fs::read_to_string(&main_rs).unwrap_or_default();
    if !main_content.contains("mod helpers;") {
        return false;
    }

    if matches!(profile, BaseProfile::WithCable) && !main_content.contains("mod channels;") {
        return false;
    }

    fs::read_to_string(&hello)
        .map(|content| content.contains("ApplicationHelper::greet"))
        .unwrap_or(false)
}

fn recreate_base(dir: &Path, profile: BaseProfile) {
    if dir.exists() {
        fs::remove_dir_all(dir).ok();
    }
    fs::create_dir_all(dir).expect("create base dir");
    doido(dir).args(profile.new_args()).assert().success();
}

fn auth_base_is_valid(dir: &Path, api: bool) -> bool {
    let app = dir.join("blog");
    let routes = app.join("config/routes.rs");
    let sessions = app.join("app/controllers/auth/sessions_controller.rs");
    let user_model = app.join("app/models/user.rs");
    let sign_in_view = app.join("app/views/auth/sign_in.html.tera");

    let sessions_content = fs::read_to_string(&sessions).unwrap_or_default();
    let routes_content = fs::read_to_string(&routes).unwrap_or_default();
    let routes_ok = routes_content.contains("SessionsController::create")
        && !routes_content.contains("use doido::auth::auth_routes;");
    let main_ok = fs::read_to_string(app.join("src/main.rs"))
        .map(|content| content.contains("mod helpers;"))
        .unwrap_or(false);
    let sessions_ok = sessions_content.contains("sign_in(ctx, &user)")
        && sessions_content.contains("doido::controller::Context")
        && if api {
            sessions_content.contains("body_json")
        } else {
            sessions_content.contains("ctx.render(\"auth/sign_in\"")
        };
    let user_ok = fs::read_to_string(&user_model)
        .map(|content| content.contains(".map_err(Into::into)") && content.contains("sea_orm::Set"))
        .unwrap_or(false);
    let cargo_ok = fs::read_to_string(app.join("Cargo.toml"))
        .map(|content| content.contains("\"auth\""))
        .unwrap_or(false);
    let seed_ok = app.join("db/seed/Cargo.toml").is_file()
        && fs::read_to_string(app.join("db/seed/Cargo.toml"))
            .map(|content| content.contains("serde ="))
            .unwrap_or(false);
    let views_ok = if api {
        !sign_in_view.exists()
    } else {
        sign_in_view.is_file()
            && app.join("app/views/auth/sign_up.html.tera").is_file()
            && fs::read_to_string(app.join("app/controllers/auth/passwords_controller.rs"))
                .map(|content| content.contains("#[allow(dead_code)]"))
                .unwrap_or(false)
    };

    app.join("Cargo.toml").is_file()
        && cargo_ok
        && seed_ok
        && routes_ok
        && sessions_ok
        && user_ok
        && views_ok
        && main_ok
}

fn recreate_auth_base(dir: &Path, profile: BaseProfile) {
    if dir.exists() {
        fs::remove_dir_all(dir).ok();
    }
    fs::create_dir_all(dir).expect("create auth base dir");
    doido(dir).args(profile.new_args()).assert().success();
}

fn ensure_base_app(profile: BaseProfile) -> PathBuf {
    static DEFAULT: OnceLock<PathBuf> = OnceLock::new();
    static API_ONLY: OnceLock<PathBuf> = OnceLock::new();
    static CABLE: OnceLock<PathBuf> = OnceLock::new();
    static JOBS_DB: OnceLock<PathBuf> = OnceLock::new();

    match profile {
        BaseProfile::WithAuthApi => {
            let dir = profile.cache_dir();
            if !auth_base_is_valid(&dir, true) {
                recreate_auth_base(&dir, profile);
            }
            dir
        }
        BaseProfile::WithAuthHtml => {
            let dir = profile.cache_dir();
            if !auth_base_is_valid(&dir, false) {
                recreate_auth_base(&dir, profile);
            }
            dir
        }
        _ => {
            let init = || {
                let dir = profile.cache_dir();
                if !base_app_is_valid(&dir, profile) {
                    recreate_base(&dir, profile);
                }
                dir
            };

            match profile {
                BaseProfile::Default => DEFAULT.get_or_init(init).clone(),
                BaseProfile::ApiOnly => API_ONLY.get_or_init(init).clone(),
                BaseProfile::WithCable => CABLE.get_or_init(init).clone(),
                BaseProfile::WithJobsDb => JOBS_DB.get_or_init(init).clone(),
                BaseProfile::WithAuthApi | BaseProfile::WithAuthHtml => unreachable!(),
            }
        }
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
