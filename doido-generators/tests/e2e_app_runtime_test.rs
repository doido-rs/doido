//! Fullest e2e (definition-of-done): scaffold an app via the CLI, migrate a
//! temp SQLite database, boot the generated app's own server, and drive a real
//! HTTP CRUD cycle against it.
//!
//! Builds the whole framework + app and boots a server, so it is `#[ignore]`d
//! and run via `make example`, never the fast `verify` gate.

use assert_cmd::Command;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use std::time::{Duration, Instant};

fn doido(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("doido-generators").expect("doido-generators binary");
    cmd.current_dir(dir);
    cmd
}

/// A scratch dir for the scaffolded app, placed under the workspace `target/`
/// (real disk, gitignored) rather than `/tmp`, which is frequently a small
/// tmpfs that a full framework + app build + migration recompile overflows.
fn e2e_tempdir() -> tempfile::TempDir {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("target/e2e");
    std::fs::create_dir_all(&base).expect("create e2e base dir");
    tempfile::Builder::new()
        .prefix("doido-e2e-")
        .tempdir_in(&base)
        .expect("create tempdir")
}

/// Grab a free TCP port by binding to :0 and releasing it.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
#[ignore = "slow: builds the framework + app and boots a server; run via `make example`"]
fn generated_app_serves_crud_over_http() {
    let tmp = e2e_tempdir();
    doido(tmp.path())
        .args(["new", "blog", "--non-interactive", "--database=sqlite"])
        .assert()
        .success();
    let app = tmp.path().join("blog");

    // A JSON API scaffold (model + controller + routes, no views).
    doido(&app)
        .args([
            "generate",
            "scaffold",
            "Post",
            "title:string",
            "body:text",
            "--api",
        ])
        .assert()
        .success();

    // Bind the server to a free port on localhost.
    let port = free_port();
    let dev_yml = app.join("config/development.yml");
    let cfg = std::fs::read_to_string(&dev_yml).unwrap();
    let cfg = cfg
        .replace("bind: 0.0.0.0", "bind: 127.0.0.1")
        .replace("port: 3000", &format!("port: {port}"));
    std::fs::write(&dev_yml, cfg).unwrap();

    // Build the app workspace against the in-tree framework.
    let status = StdCommand::new(env!("CARGO"))
        .args(["build", "--workspace", "--manifest-path"])
        .arg(app.join("Cargo.toml"))
        .status()
        .expect("cargo build");
    assert!(status.success(), "generated app failed to build");

    let bin = app.join("target/debug/blog");

    // Create + migrate the SQLite database.
    run_app(&bin, &app, &["db", "create"]);
    run_app(&bin, &app, &["db", "migrate"]);

    // Boot the server as a child process.
    let mut server = StdCommand::new(&bin)
        .arg("server")
        .current_dir(&app)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn server");

    let result = std::panic::catch_unwind(|| crud_cycle(port));
    let _ = server.kill();
    let _ = server.wait();
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

fn run_app(bin: &Path, app: &Path, args: &[&str]) {
    let status = StdCommand::new(bin)
        .args(args)
        .current_dir(app)
        .status()
        .unwrap_or_else(|e| panic!("run {args:?}: {e}"));
    assert!(status.success(), "app command {args:?} failed");
}

fn crud_cycle(port: u16) {
    let base = format!("http://127.0.0.1:{port}");
    wait_until_ready(&base);

    // CREATE
    let created: serde_json::Value = ureq::post(&format!("{base}/posts"))
        .send_json(serde_json::json!({ "title": "Hello", "body": "world" }))
        .expect("POST /posts")
        .into_body()
        .read_json()
        .expect("json body");
    let id = created["id"].as_i64().expect("created id");
    assert_eq!(created["title"], "Hello");

    // INDEX
    let index: serde_json::Value = ureq::get(&format!("{base}/posts"))
        .call()
        .expect("GET /posts")
        .into_body()
        .read_json()
        .unwrap();
    assert!(
        index
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["title"] == "Hello"),
        "index lists the created post"
    );

    // SHOW
    assert_eq!(
        ureq::get(&format!("{base}/posts/{id}"))
            .call()
            .unwrap()
            .status()
            .as_u16(),
        200
    );

    // UPDATE
    let updated: serde_json::Value = ureq::patch(&format!("{base}/posts/{id}"))
        .send_json(serde_json::json!({ "title": "Updated", "body": "world" }))
        .expect("PATCH /posts/:id")
        .into_body()
        .read_json()
        .unwrap();
    assert_eq!(updated["title"], "Updated");

    // DELETE
    assert_eq!(
        ureq::delete(&format!("{base}/posts/{id}"))
            .call()
            .expect("DELETE /posts/:id")
            .status()
            .as_u16(),
        204
    );
}

fn wait_until_ready(base: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if ureq::get(&format!("{base}/posts")).call().is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("server did not become ready within 30s");
}
